// Utility functions for player state management

import { LyrionAPI } from './api';
import { useAppStore } from './store';
import type { Track } from '../types/api';

/**
 * Fetch the current player status and update the store
 * This includes current track, position, volume, playing state, and playlist
 */
export async function updatePlayerStatus(playerId: string) {
  console.log('[PlayerUtils] Fetching status for player:', playerId);
  try {
    // Fetch status (which now includes playlist)
    const statusResponse = await LyrionAPI.getStatus(playerId);
    console.log('[PlayerUtils] Status response:', statusResponse);
    const status = statusResponse.result as any;

    if (status && status.playlist_loop && status.playlist_loop.length > 0) {
      const currentTrack = status.playlist_loop[status.playlist_cur_index || 0];
      console.log('[PlayerUtils] Current track from status:', currentTrack);

      // Fetch full track details from database to get duration
      const tracks = useAppStore.getState().tracks;
      const trackDetails = tracks.find((t: Track) => t.id === currentTrack.id);
      console.log('[PlayerUtils] Track details from store:', trackDetails);

      // Update nowPlaying with all info
      useAppStore.getState().setNowPlaying({
        track: trackDetails || {
          id: currentTrack.id,
          url: '',
          title: currentTrack.title || 'Unknown Track',
          artist: currentTrack.artist || 'Unknown Artist',
          album: currentTrack.album || 'Unknown Album',
        } as Track,
        playlist: status.playlist_loop,
        playlist_index: status.playlist_cur_index || 0,
        duration: trackDetails?.secs || 180, // Default to 3 minutes if unknown
        playing: status.mode === 'play',
        position: status.time || 0,
        volume: status.mixer_volume !== undefined ? status.mixer_volume : 50,
      });

      console.log('[PlayerUtils] Store updated with track:', trackDetails?.title, 'duration:', trackDetails?.secs);
    } else if (status) {
      // No tracks in playlist or stopped - clear playing state
      console.log('[PlayerUtils] No tracks in playlist, clearing state');
      useAppStore.getState().setNowPlaying({
        track: undefined,
        playlist: [],
        playlist_index: 0,
        duration: 0,
        playing: false,
        position: 0,
        volume: status.mixer_volume !== undefined ? status.mixer_volume : 50,
      });
    }
  } catch (error) {
    console.error('[PlayerUtils] Failed to update player status:', error);
  }
}

/**
 * Play a single track and update UI
 */
export async function playTrack(playerId: string, trackId: number) {
  console.log('[PlayerUtils] Playing track:', trackId, 'on player:', playerId);

  // Use direct play command with track ID
  const response = await LyrionAPI.jsonrpc(playerId, ['play', trackId]);
  console.log('[PlayerUtils] Play response:', response);

  // Give backend a moment to process, then fetch status
  setTimeout(() => {
    console.log('[PlayerUtils] Fetching status after play...');
    updatePlayerStatus(playerId);
  }, 500);
}

/**
 * Play multiple tracks and update UI
 */
export async function playTracks(playerId: string, trackIds: number[]) {
  console.log('[PlayerUtils] Playing tracks:', trackIds, 'on player:', playerId);

  // Clear playlist first
  await LyrionAPI.clearPlaylist(playerId);
  console.log('[PlayerUtils] Playlist cleared');

  // Add all tracks
  for (const trackId of trackIds) {
    await LyrionAPI.addTrack(playerId, trackId);
    console.log('[PlayerUtils] Added track:', trackId);
  }

  // Play the first track
  if (trackIds.length > 0) {
    const response = await LyrionAPI.jsonrpc(playerId, ['play', trackIds[0]]);
    console.log('[PlayerUtils] Play response:', response);
  }

  // Give backend a moment to process, then fetch status
  setTimeout(() => {
    console.log('[PlayerUtils] Fetching status after play...');
    updatePlayerStatus(playerId);
  }, 500);
}
