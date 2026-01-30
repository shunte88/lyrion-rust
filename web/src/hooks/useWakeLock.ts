import { useEffect, useRef } from 'react';
import { useAppStore } from '../services/store';

/**
 * Hook to manage wake lock to prevent screen from sleeping during playback
 */
export function useWakeLock() {
  const { nowPlaying } = useAppStore();
  const wakeLockRef = useRef<WakeLockSentinel | null>(null);

  useEffect(() => {
    // Check if Wake Lock API is supported
    if (!('wakeLock' in navigator)) {
      console.log('Wake Lock API not supported');
      return;
    }

    const requestWakeLock = async () => {
      try {
        // Request wake lock
        wakeLockRef.current = await navigator.wakeLock.request('screen');
        console.log('Wake lock acquired');

        // Handle wake lock release (e.g., when tab becomes hidden)
        wakeLockRef.current.addEventListener('release', () => {
          console.log('Wake lock released');
        });
      } catch (error) {
        console.error('Failed to acquire wake lock:', error);
      }
    };

    const releaseWakeLock = async () => {
      if (wakeLockRef.current) {
        try {
          await wakeLockRef.current.release();
          wakeLockRef.current = null;
          console.log('Wake lock released manually');
        } catch (error) {
          console.error('Failed to release wake lock:', error);
        }
      }
    };

    // Request wake lock when playing, release when paused/stopped
    if (nowPlaying.playing && nowPlaying.track) {
      requestWakeLock();
    } else {
      releaseWakeLock();
    }

    // Handle visibility change (reacquire wake lock when tab becomes visible again)
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible' && nowPlaying.playing && nowPlaying.track) {
        requestWakeLock();
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);

    // Cleanup
    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      releaseWakeLock();
    };
  }, [nowPlaying.playing, nowPlaying.track]);
}
