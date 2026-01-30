import { useEffect } from 'react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';

/**
 * Hook to integrate with Media Session API for native media controls
 * Provides controls in lock screen, notification center, and car displays
 */
export function useMediaSession() {
  const { nowPlaying, currentPlayer } = useAppStore();

  useEffect(() => {
    if (!('mediaSession' in navigator)) {
      console.log('Media Session API not supported');
      return;
    }

    // Update metadata when track changes
    if (nowPlaying.track) {
      const coverUrl = nowPlaying.track.has_cover && nowPlaying.track.id
        ? LyrionAPI.getCoverArtUrl(nowPlaying.track.id)
        : undefined;

      navigator.mediaSession.metadata = new MediaMetadata({
        title: nowPlaying.track.title || 'Unknown Title',
        artist: nowPlaying.track.artist || 'Unknown Artist',
        album: nowPlaying.track.album || 'Unknown Album',
        artwork: coverUrl
          ? [
              { src: coverUrl, sizes: '96x96', type: 'image/jpeg' },
              { src: coverUrl, sizes: '128x128', type: 'image/jpeg' },
              { src: coverUrl, sizes: '192x192', type: 'image/jpeg' },
              { src: coverUrl, sizes: '256x256', type: 'image/jpeg' },
              { src: coverUrl, sizes: '384x384', type: 'image/jpeg' },
              { src: coverUrl, sizes: '512x512', type: 'image/jpeg' },
            ]
          : [],
      });
    } else {
      navigator.mediaSession.metadata = null;
    }

    // Update playback state
    navigator.mediaSession.playbackState = nowPlaying.playing ? 'playing' : 'paused';

    // Set up action handlers
    const handlePlay = async () => {
      if (currentPlayer) {
        await LyrionAPI.play(currentPlayer.id);
      }
    };

    const handlePause = async () => {
      if (currentPlayer) {
        await LyrionAPI.pause(currentPlayer.id);
      }
    };

    const handlePreviousTrack = async () => {
      if (currentPlayer) {
        await LyrionAPI.previous(currentPlayer.id);
      }
    };

    const handleNextTrack = async () => {
      if (currentPlayer) {
        await LyrionAPI.next(currentPlayer.id);
      }
    };

    const handleSeekTo = async (details: MediaSessionActionDetails) => {
      if (currentPlayer && details.seekTime !== undefined) {
        // TODO: Implement seek to position API call
        console.log('Seek to:', details.seekTime);
      }
    };

    const handleSeekBackward = async (details: MediaSessionActionDetails) => {
      if (currentPlayer) {
        const seekOffset = details.seekOffset || 10; // Default 10 seconds
        // TODO: Implement seek backward API call
        console.log('Seek backward:', seekOffset);
      }
    };

    const handleSeekForward = async (details: MediaSessionActionDetails) => {
      if (currentPlayer) {
        const seekOffset = details.seekOffset || 10; // Default 10 seconds
        // TODO: Implement seek forward API call
        console.log('Seek forward:', seekOffset);
      }
    };

    // Register action handlers
    navigator.mediaSession.setActionHandler('play', handlePlay);
    navigator.mediaSession.setActionHandler('pause', handlePause);
    navigator.mediaSession.setActionHandler('previoustrack', handlePreviousTrack);
    navigator.mediaSession.setActionHandler('nexttrack', handleNextTrack);

    // Optional: Seek handlers (if supported)
    try {
      navigator.mediaSession.setActionHandler('seekto', handleSeekTo);
      navigator.mediaSession.setActionHandler('seekbackward', handleSeekBackward);
      navigator.mediaSession.setActionHandler('seekforward', handleSeekForward);
    } catch (error) {
      console.log('Seek actions not supported:', error);
    }

    // Update position state (if supported)
    if ('setPositionState' in navigator.mediaSession) {
      try {
        if (nowPlaying.duration > 0) {
          navigator.mediaSession.setPositionState({
            duration: nowPlaying.duration,
            playbackRate: 1.0,
            position: nowPlaying.position,
          });
        }
      } catch (error) {
        console.log('Failed to set position state:', error);
      }
    }

    // Cleanup
    return () => {
      if ('mediaSession' in navigator) {
        navigator.mediaSession.metadata = null;
        navigator.mediaSession.setActionHandler('play', null);
        navigator.mediaSession.setActionHandler('pause', null);
        navigator.mediaSession.setActionHandler('previoustrack', null);
        navigator.mediaSession.setActionHandler('nexttrack', null);
        navigator.mediaSession.setActionHandler('seekto', null);
        navigator.mediaSession.setActionHandler('seekbackward', null);
        navigator.mediaSession.setActionHandler('seekforward', null);
      }
    };
  }, [nowPlaying, currentPlayer]);
}
