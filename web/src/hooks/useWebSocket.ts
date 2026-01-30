// React hook for WebSocket connection and real-time updates

import { useEffect, useState } from 'react';
import { websocketService, type WebSocketMessage } from '../services/websocket';
import { useAppStore } from '../services/store';

export function useWebSocket() {
  const [connected, setConnected] = useState(false);
  const { setNowPlaying, currentPlayer } = useAppStore();

  useEffect(() => {
    // Connect to WebSocket (with error handling)
    try {
      websocketService.connect();
    } catch (error) {
      console.error('[useWebSocket] Failed to connect:', error);
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

    // Cleanup
    return () => {
      unsubscribeConnection();
      unsubscribeMessages();
      clearInterval(progressInterval);
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
