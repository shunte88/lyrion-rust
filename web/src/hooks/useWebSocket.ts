// React hook for WebSocket connection and real-time updates

import { useEffect, useState, useRef } from 'react';
import { websocketService, type WebSocketMessage } from '../services/websocket';
import { useAppStore } from '../services/store';
import { updatePlayerStatus } from '../services/playerUtils';

export function useWebSocket() {
  const [connected, setConnected] = useState(false);
  const { setNowPlaying, currentPlayer } = useAppStore();
  const prevPlayerIdRef = useRef<string | null>(null);

  // Watch for player changes and fetch status
  useEffect(() => {
    if (currentPlayer && currentPlayer.id) {
      if (currentPlayer.id !== prevPlayerIdRef.current) {
        console.log('[useWebSocket] Player changed, fetching status for:', currentPlayer.id);
        prevPlayerIdRef.current = currentPlayer.id;
        // Fetch status for the newly selected player
        updatePlayerStatus(currentPlayer.id);
      }
    }
  }, [currentPlayer]);

  useEffect(() => {
    // Connect to WebSocket (with error handling)
    try {
      websocketService.connect();
    } catch (error) {
      console.error('[useWebSocket] Failed to connect:', error);
    }

    // Fetch initial status if a player is already selected (on page load/refresh)
    if (currentPlayer && currentPlayer.id) {
      console.log('[useWebSocket] Initial mount - fetching status for current player:', currentPlayer.id);
      updatePlayerStatus(currentPlayer.id);
    }

    // Subscribe to connection status
    const unsubscribeConnection = websocketService.onConnectionChange((isConnected) => {
      setConnected(isConnected);
    });

    // Subscribe to messages
    const unsubscribeMessages = websocketService.subscribe((message: WebSocketMessage) => {
      handleMessage(message);
    });

    // Progress ticker - update position every 950ms when playing
    const progressInterval = setInterval(() => {
      if (currentPlayer) {
        useAppStore.setState((state) => {
          if (state.nowPlaying.playing && state.nowPlaying.duration > 0) {
            const newPosition = Math.min(
              state.nowPlaying.position + 0.95,
              state.nowPlaying.duration
            );

            // Check if track completed
            if (newPosition >= state.nowPlaying.duration - 0.5) {
              // Track is about to end, poll status to see what's next
              console.log('[useWebSocket] Track completion detected, polling status');
              if (currentPlayer.id) {
                updatePlayerStatus(currentPlayer.id);
              }
            }

            return {
              nowPlaying: {
                ...state.nowPlaying,
                position: newPosition,
              },
            };
          }
          return state;
        });
      }
    }, 950);

    // Periodic status polling - refresh every 10 seconds to catch state changes
    const statusPollInterval = setInterval(() => {
      if (currentPlayer && currentPlayer.id) {
        console.log('[useWebSocket] Periodic status poll');
        updatePlayerStatus(currentPlayer.id);
      }
    }, 10000);

    // Cleanup
    return () => {
      unsubscribeConnection();
      unsubscribeMessages();
      clearInterval(progressInterval);
      clearInterval(statusPollInterval);
      websocketService.disconnect();
    };
  }, [currentPlayer]);

  const handleMessage = (message: WebSocketMessage) => {
    switch (message.type) {
      case 'player_status':
        handlePlayerStatus(message.data);
        break;

      case 'player_connected':
        console.log('[WebSocket] Player connected:', message.data);
        // Update players list
        useAppStore.setState((state) => {
          const existingIndex = state.players.findIndex((p) => p.id === message.data.id);
          if (existingIndex >= 0) {
            const updated = [...state.players];
            updated[existingIndex] = message.data;
            return { players: updated };
          } else {
            return { players: [...state.players, message.data] };
          }
        });
        break;

      case 'player_disconnected':
        console.log('[WebSocket] Player disconnected:', message.data.player_id);
        useAppStore.setState((state) => ({
          players: state.players.filter((p) => p.id !== message.data.player_id),
        }));
        break;

      case 'track_started':
        handleTrackStarted(message.data);
        break;

      case 'progress_update':
        handleProgressUpdate(message.data);
        break;

      default:
        console.warn('[WebSocket] Unknown message type:', message);
    }
  };

  const handlePlayerStatus = (data: any) => {
    // Only update if this is the current player
    if (currentPlayer && data.player_id === currentPlayer.id) {
      setNowPlaying({
        playing: data.playing,
        position: data.position,
        volume: data.volume,
      });
    }
  };

  const handleTrackStarted = (data: any) => {
    if (currentPlayer && data.player_id === currentPlayer.id) {
      // TODO: Fetch full track details from API
      setNowPlaying({
        position: 0,
        duration: data.duration || 0,
        playing: true,
      });
    }
  };

  const handleProgressUpdate = (data: any) => {
    if (currentPlayer && data.player_id === currentPlayer.id) {
      setNowPlaying({
        position: data.position,
        duration: data.duration,
      });
    }
  };

  return {
    connected,
    send: websocketService.send.bind(websocketService),
  };
}
