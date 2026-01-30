//! Lyrion Music Server main binary
//! Complete rewrite from Perl to Rust

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path, Request},
    Json,
    response::IntoResponse,
    http::{StatusCode, Method},
    body::Body,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod jsonrpc;
mod streaming;
mod player_manager;
mod sync_coordinator;
mod websocket;
mod playlist;

use lyrion_core::SyncManager;
use lyrion_db::{DatabaseConfig, initialize_database};
use lyrion_protocol::{SlimprotoServer, SlimprotoMessage, DiscoveryServer};
use lyrion_plugins::{PluginManager, PluginConfig, PluginContext};
use sync_coordinator::SyncCoordinator;

/// Server state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::SqlitePool,
    pub slimproto_server: Arc<SlimprotoServer>,
    pub sync_manager: Arc<SyncManager>,
    pub sync_coordinator: Arc<SyncCoordinator>,
    pub plugin_manager: Arc<RwLock<PluginManager>>,
    pub playlist_manager: Arc<playlist::PlaylistManager>,
    pub ws_broadcast: broadcast::Sender<websocket::WsMessage>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lyrion_server=debug,lyrion_protocol=debug,lyrion_db=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Lyrion Music Server");

    // Initialize database
    let db_config = DatabaseConfig {
        path: "lyrion-rust.db".to_string(),
        max_connections: 10,
    };

    tracing::info!("Initializing database at {}", db_config.path);
    let db_pool = initialize_database(&db_config).await?;

    // Create Slimproto server
    let (slimproto_server, mut message_rx) = SlimprotoServer::new();
    let slimproto_server = Arc::new(slimproto_server);

    // Spawn Slimproto server task
    let slimproto_clone = Arc::clone(&slimproto_server);
    tokio::spawn(async move {
        if let Err(e) = slimproto_clone.listen("0.0.0.0").await {
            tracing::error!("Slimproto server error: {}", e);
        }
    });

    // Spawn UDP discovery server task
    tokio::spawn(async move {
        match DiscoveryServer::bind(
            "0.0.0.0",
            3483,
            "Lyrion Music Server".to_string(),
            uuid::Uuid::new_v4().to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            9000,
        )
        .await
        {
            Ok(discovery_server) => {
                if let Err(e) = discovery_server.run().await {
                    tracing::error!("UDP discovery server error: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to start UDP discovery server: {}", e);
            }
        }
    });

    // Create sync manager and coordinator
    let sync_manager = Arc::new(SyncManager::new());
    let sync_coordinator = Arc::new(SyncCoordinator::new(Arc::clone(&sync_manager)));

    // Start sync loop (950ms interval)
    let sync_coordinator_clone = Arc::clone(&sync_coordinator);
    tokio::spawn(async move {
        sync_coordinator_clone.start_sync_loop().await;
    });

    // Initialize plugin system
    tracing::info!("Initializing plugin system");
    let plugin_config = PluginConfig {
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: PathBuf::from("."),
        plugin_dir: PathBuf::from("plugins-deployed"),
        base_url: "http://localhost:9000".to_string(),
    };

    let mut plugin_manager = PluginManager::new(plugin_config.clone());

    // Discover plugins
    match plugin_manager.discover() {
        Ok(plugins) => {
            tracing::info!("Discovered {} plugins: {:?}", plugins.len(), plugins);

            // Create plugin context
            let plugin_context = PluginContext {
                db_pool: db_pool.clone(),
                config: plugin_config,
                preferences: HashMap::new(),
            };

            // Load all plugins
            unsafe {
                match plugin_manager.load_all(&plugin_context) {
                    Ok(loaded) => {
                        tracing::info!("Successfully loaded {} plugins: {:?}", loaded.len(), loaded);
                    }
                    Err(e) => {
                        tracing::error!("Failed to load plugins: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!("Plugin discovery failed: {}", e);
        }
    }

    let plugin_manager = Arc::new(RwLock::new(plugin_manager));

    // Create WebSocket broadcast channel (before message handler so it can use it)
    let (ws_tx, _) = broadcast::channel::<websocket::WsMessage>(100);

    // Spawn message handler task
    let sync_manager_clone = Arc::clone(&sync_manager);
    let ws_broadcast_clone = ws_tx.clone();
    let slimproto_clone_for_msg = Arc::clone(&slimproto_server);
    tokio::spawn(async move {
        use std::collections::HashMap;
        use std::time::Instant;

        // Track last broadcast time per player for debouncing
        let mut last_broadcast: HashMap<String, Instant> = HashMap::new();

        while let Some((mac, message)) = message_rx.recv().await {
            tracing::debug!("Processing message from {}: {:?}", mac, message);

            // Handle STAT messages to update play points and broadcast progress
            if let SlimprotoMessage::Stat(stat) = &message {
                // Extract position from STAT message
                let position_secs = stat.elapsed_seconds as f64 +
                                   (stat.elapsed_milliseconds as f64 / 1000.0);

                // Get player ID from MAC - look up from get_players()
                let players = slimproto_clone_for_msg.get_players().await;
                if let Some((_, player_conn)) = players.iter().find(|(player_mac, _)| player_mac == &mac) {
                    if let Some(ref uuid) = player_conn.uuid {
                        let player_id = uuid.clone();

                        // Debounce: only broadcast if > 950ms since last update
                        let should_broadcast = last_broadcast
                            .get(&player_id)
                            .map(|last| last.elapsed().as_millis() > 950)
                            .unwrap_or(true);

                        if should_broadcast {
                            // Broadcast progress update
                            let _ = ws_broadcast_clone.send(websocket::WsMessage::ProgressUpdate(
                                websocket::ProgressUpdateEvent {
                                    player_id: player_id.clone(),
                                    position: position_secs,
                                    duration: 0.0, // TODO: Get actual duration from track
                                }
                            ));

                            last_broadcast.insert(player_id, Instant::now());
                        }
                    }
                }

                tracing::debug!("STAT message: elapsed={}s {}ms",
                    stat.elapsed_seconds, stat.elapsed_milliseconds);
            }
        }
    });

    // Create app state
    // Initialize playlist manager
    let playlist_manager = Arc::new(playlist::PlaylistManager::new());

    let app_state = AppState {
        db_pool,
        slimproto_server,
        sync_manager,
        sync_coordinator,
        plugin_manager,
        playlist_manager,
        ws_broadcast: ws_tx,
    };

    // Build HTTP router
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/api/v1/players", get(api::list_players))
        .route("/api/v1/tracks", get(api::list_tracks))
        .route("/api/v1/tracks/search", get(api::search_tracks))
        .route("/api/v1/cover/:track_id", get(api::get_cover_art))
        .route("/stream/:track_id", get(streaming::stream_track))
        .route("/stream/:track_id/icy", get(streaming::stream_with_icy))
        .route("/jsonrpc.js", post(jsonrpc::jsonrpc_handler))
        .route("/ws", get(websocket::websocket_handler))
        // Plugin routes - catch all requests to /plugins/*
        .route("/plugins/*path",
            get(plugin_handler)
                .post(plugin_handler)
                .put(plugin_handler)
                .delete(plugin_handler)
                .patch(plugin_handler))
        .nest_service("/static", ServeDir::new("web/dist"))
        .with_state(app_state);

    // Start HTTP server
    let http_addr = "0.0.0.0:9000";
    tracing::info!("HTTP server listening on {}", http_addr);

    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Plugin request handler
async fn plugin_handler(
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let query_str = req.uri().query().map(|s| s.to_string());

    tracing::debug!("Plugin request: {} {}", method, path);

    // Extract query parameters
    let query: HashMap<String, String> = query_str
        .as_ref()
        .map(|q| {
            url::form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default();

    // Extract headers
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str().ok().map(|val| (k.as_str().to_string(), val.to_string()))
        })
        .collect();

    // Read body
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes.to_vec(),
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {}", e),
            )
                .into_response();
        }
    };

    // Find handler for this route
    let plugin_info = {
        let plugin_manager = state.plugin_manager.read().await;
        let registry = plugin_manager.registry();
        registry.route_http_request(&method, &path)
            .map(|(name, handler)| (name.to_string(), handler.to_string()))
    };

    if let Some((plugin_name, handler_id)) = plugin_info {
        tracing::debug!("Routing to plugin: {} (handler: {})", plugin_name, handler_id);

        // Get write lock to call plugin
        let mut plugin_manager = state.plugin_manager.write().await;

        if let Some(loaded_plugin) = plugin_manager.get_plugin_mut(&plugin_name) {
            // Create plugin request
            let plugin_request = lyrion_plugins::HttpRequest {
                method: method.clone(),
                path: path.clone(),
                headers,
                body: body_bytes,
                query,
            };

            // Call plugin handler
            match loaded_plugin.plugin_mut().handle_http_request(plugin_request) {
                Ok(response) => {
                    // Convert plugin response to Axum response
                    let mut axum_response = (StatusCode::from_u16(response.status).unwrap_or(StatusCode::OK), response.body).into_response();

                    // Add headers
                    let response_headers = axum_response.headers_mut();
                    for (key, value) in response.headers {
                        if let Ok(header_name) = axum::http::HeaderName::from_bytes(key.as_bytes()) {
                            if let Ok(header_value) = axum::http::HeaderValue::from_str(&value) {
                                response_headers.insert(header_name, header_value);
                            }
                        }
                    }

                    axum_response
                }
                Err(e) => {
                    tracing::error!("Plugin handler error: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, format!("Plugin error: {}", e))
                        .into_response()
                }
            }
        } else {
            tracing::error!("Plugin not found: {}", plugin_name);
            (
                StatusCode::NOT_FOUND,
                format!("Plugin not found: {}", plugin_name),
            )
                .into_response()
        }
    } else {
        tracing::debug!("No plugin handler found for: {} {}", method, path);
        (
            StatusCode::NOT_FOUND,
            format!("No plugin handler for {} {}", method, path),
        )
            .into_response()
    }
}

/// Root handler
async fn root_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        "Lyrion Music Server\nRust Edition v0.1.0\n\nEndpoints:\n  /api/v1/players\n  /api/v1/tracks\n  /jsonrpc.js\n  /plugins/* (dynamic plugin routes)\n",
    )
}
