/**
 * Haptic feedback utilities for mobile devices
 */

type HapticPattern = number | number[];

/**
 * Trigger haptic feedback vibration
 * @param pattern - Vibration pattern (ms) or array of [vibrate, pause, vibrate, ...]
 */
export function vibrate(pattern: HapticPattern): void {
  if ('vibrate' in navigator) {
    try {
      navigator.vibrate(pattern);
    } catch (error) {
      console.debug('Vibration failed:', error);
    }
  }
}

/**
 * Stop any ongoing vibration
 */
export function stopVibration(): void {
  if ('vibrate' in navigator) {
    try {
      navigator.vibrate(0);
    } catch (error) {
      console.debug('Stop vibration failed:', error);
    }
  }
}

/**
 * Predefined haptic patterns for common interactions
 */
export const Haptics = {
  /**
   * Light tap feedback for button presses
   */
  tap: () => vibrate(10),

  /**
   * Medium feedback for important actions
   */
  impact: () => vibrate(15),

  /**
   * Strong feedback for critical actions
   */
  heavy: () => vibrate(25),

  /**
   * Success pattern
   */
  success: () => vibrate([5, 10, 5]),

  /**
   * Error pattern
   */
  error: () => vibrate([10, 20, 10]),

  /**
   * Warning pattern
   */
  warning: () => vibrate([15, 10, 15]),

  /**
   * Selection change (like switching tabs)
   */
  selection: () => vibrate(8),

  /**
   * Notification received
   */
  notification: () => vibrate([10, 50, 10, 50, 10]),
};

/**
 * Check if haptic feedback is supported
 */
export function isHapticSupported(): boolean {
  return 'vibrate' in navigator;
}
