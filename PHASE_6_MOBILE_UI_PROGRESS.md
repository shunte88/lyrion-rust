# Phase 6: Mobile UI & PWA - In Progress 🚀

## Overview
Making Lyrion Music Server a fully installable Progressive Web App with mobile-optimized UI.

## Completed So Far ✅

### 1. PWA Foundation ✅

**Files Created/Modified**:
- ✅ `web/public/manifest.json` - PWA manifest with app metadata
- ✅ `web/index.html` - Added PWA meta tags, iOS/Android support
- ✅ `web/vite.config.ts` - Configured vite-plugin-pwa with Workbox
- ✅ `web/package.json` - Added vite-plugin-pwa dependency

**PWA Manifest Features**:
```json
{
  "name": "Lyrion Music Server",
  "short_name": "Lyrion",
  "display": "standalone",
  "theme_color": "#228be6",
  "background_color": "#1a1b1e",
  "shortcuts": [
    "Now Playing",
    "Library"
  ]
}
```

**Meta Tags Added**:
- ✅ `viewport-fit=cover` - Safe area support for notched devices
- ✅ `user-scalable=no` - Prevent zoom on mobile
- ✅ `theme-color` - Status bar theming
- ✅ `apple-mobile-web-app-capable` - iOS home screen install
- ✅ `apple-mobile-web-app-status-bar-style` - iOS status bar style

### 2. Service Worker with Workbox ✅

**Caching Strategy**:
1. **API Calls** - `NetworkFirst` (always try network, fall back to cache)
   - Cache name: `api-cache`
   - Max entries: 100
   - Max age: 24 hours

2. **Cover Art** - `CacheFirst` (serve from cache if available)
   - Cache name: `cover-art-cache`
   - Max entries: 500
   - Max age: 30 days

**Offline Support**:
- ✅ Offline fallback to `/index.html`
- ✅ API/WebSocket/Stream excluded from caching
- ✅ Automatic service worker updates

**Build Output**:
```
PWA v1.2.0
mode      generateSW
precache  4 entries (636.06 KiB)
files generated
  dist/sw.js
  dist/workbox-4b126c97.js
```

### 3. Backend Server Fixed ✅

**Issue Resolved**: UI was throwing exceptions because backend wasn't running
**Solution**: Started lyrion-server on port 9000
**Status**: ✅ API responding with 50 tracks
**Logs**: Plugin system loaded successfully (RandomPlay plugin active)

## Still TODO ⏳

### 1. Create PWA Icons 📱
**Status**: Placeholder created
**Needed**:
- `icon-192.png` (192x192 - standard)
- `icon-512.png` (512x512 - high-res)
- `icon-192-maskable.png` (192x192 - with safe zone)
- `icon-512-maskable.png` (512x512 - with safe zone)
- `icon-play.png` (96x96 - shortcut icon)
- `icon-library.png` (96x96 - shortcut icon)

**Design Requirements**:
- Simple, recognizable Lyrion logo
- Works on light and dark backgrounds
- Maskable icons need 20% safe zone padding

### 2. Mobile-Responsive Layout 📐

**Current Status**: Desktop-optimized
**Changes Needed**:

**a) Responsive Breakpoints**:
```typescript
// Mantine breakpoints
const theme = {
  breakpoints: {
    xs: '36em',  // 576px - phone portrait
    sm: '48em',  // 768px - phone landscape / small tablet
    md: '62em',  // 992px - tablet
    lg: '75em',  // 1200px - desktop
    xl: '88em',  // 1408px - large desktop
  }
}
```

**b) Sidebar → Bottom Navigation**:
- Current: Left sidebar (desktop)
- Mobile: Bottom tab bar with 4-5 items
- Icons only on mobile, icons+text on tablet+

**c) Album Grid Optimization**:
- Mobile: 2 columns
- Tablet: 3-4 columns
- Desktop: 5-6 columns (already implemented)

**d) Track List → Cards**:
- Mobile: Card-based layout (easier to tap)
- Desktop: Table view (current)

### 3. Touch-Optimized Controls 👆

**Minimum Tap Target**: 44x44px (Apple HIG / Material Design)

**Changes Needed**:
- ✅ Buttons already use Mantine (good default size)
- ⏳ Increase icon button size on mobile
- ⏳ Add padding around clickable elements
- ⏳ Larger play/pause controls in Now Playing
- ⏳ Touch-friendly volume slider (bigger thumb)

### 4. Bottom Sheet UI 📋

**Use Cases**:
1. **Now Playing** - Slide up to see full player
2. **Queue Management** - View/edit playlist
3. **Track Options** - Add to playlist, share, etc.
4. **Player Selection** - Choose active player

**Implementation**:
- Library: `react-spring` for animations
- Gestures: Swipe up/down to open/close
- Background dimming when open
- Snap points: minimized → half → full

**Example Component**:
```typescript
<BottomSheet
  open={isOpen}
  onClose={() => setIsOpen(false)}
  snapPoints={[0, 300, '100%']}
>
  <NowPlaying />
</BottomSheet>
```

### 5. Swipe Gestures 👉

**Gestures to Implement**:
1. **Track List**: Swipe right/left for quick actions
   - Swipe right: Add to queue
   - Swipe left: Options menu
2. **Now Playing**: Swipe left/right to skip tracks
3. **Album Grid**: Swipe down to refresh

**Library**: `react-swipeable` or `framer-motion`

### 6. Media Session API 🎵

**Purpose**: Native media controls in:
- Lock screen
- Notification center
- Control center (iOS)
- Bluetooth devices
- Car displays

**Implementation**:
```typescript
if ('mediaSession' in navigator) {
  navigator.mediaSession.metadata = new MediaMetadata({
    title: 'Song Title',
    artist: 'Artist Name',
    album: 'Album Name',
    artwork: [
      { src: '/cover.png', sizes: '512x512', type: 'image/png' }
    ]
  });

  navigator.mediaSession.setActionHandler('play', () => {
    // Play handler
  });

  navigator.mediaSession.setActionHandler('pause', () => {
    // Pause handler
  });

  navigator.mediaSession.setActionHandler('previoustrack', () => {
    // Previous track
  });

  navigator.mediaSession.setActionHandler('nexttrack', () => {
    // Next track
  });
}
```

### 7. Wake Lock API 🔒

**Purpose**: Prevent screen from sleeping during playback

**Implementation**:
```typescript
let wakeLock: WakeLockSentinel | null = null;

const enableWakeLock = async () => {
  try {
    wakeLock = await navigator.wakeLock.request('screen');
  } catch (err) {
    console.error('Wake Lock failed:', err);
  }
};

// Enable when playing, release when paused
```

### 8. Haptic Feedback ⚡

**Purpose**: Tactile feedback for actions

**Implementation**:
```typescript
const vibrate = (pattern: number | number[]) => {
  if ('vibrate' in navigator) {
    navigator.vibrate(pattern);
  }
};

// Examples:
vibrate(10); // Short tap
vibrate([10, 20, 10]); // Pattern for errors
```

**Use Cases**:
- Button presses: 10ms
- Track skip: 15ms
- Error: [10, 20, 10]
- Success: [5, 10, 5]

## Architecture Changes Needed

### 1. Responsive Sidebar Component

**Current**: `src/components/Sidebar.tsx`
**Changes**:
```typescript
// New: MobileNav.tsx
export function MobileNav() {
  return (
    <Group position="apart" className="mobile-nav">
      <NavLink icon={<IconHome />} href="/" />
      <NavLink icon={<IconDisc />} href="/albums" />
      <NavLink icon={<IconMusic />} href="/tracks" />
      <NavLink icon={<IconPlaylist />} href="/playlists" />
      <NavLink icon={<IconSettings />} href="/settings" />
    </Group>
  );
}

// Update App.tsx to conditionally render
const isMobile = useMediaQuery('(max-width: 768px)');
{isMobile ? <MobileNav /> : <Sidebar />}
```

### 2. Now Playing Bottom Sheet

**New File**: `src/components/NowPlayingSheet.tsx`
**Features**:
- Minimized bar at bottom (album art + title + play/pause)
- Expand to half-height (lyrics, queue preview)
- Expand to full-height (full controls, volume, etc.)

### 3. Touch-Friendly Track Card

**New File**: `src/components/TrackCard.tsx`
**Mobile Layout**:
```
┌─────────────────────────────────┐
│ [Cover]  Song Title             │
│ [Art  ]  Artist Name            │
│          ⋮                   [▶]│
└─────────────────────────────────┘
```

## Testing Checklist

### Desktop (Already Working)
- ✅ Sidebar navigation
- ✅ Album grid
- ✅ Track table
- ✅ Search
- ✅ Settings

### Mobile (To Test)
- ⏳ Install PWA on iOS
- ⏳ Install PWA on Android
- ⏳ Offline functionality
- ⏳ Bottom navigation
- ⏳ Bottom sheet player
- ⏳ Swipe gestures
- ⏳ Media Session controls
- ⏳ Wake lock during playback
- ⏳ Haptic feedback

### Tablet (To Test)
- ⏳ Responsive layout
- ⏳ Touch controls
- ⏳ Orientation changes

## Performance Targets

- **First Contentful Paint**: < 1s
- **Time to Interactive**: < 2s
- **Lighthouse PWA Score**: > 90
- **Touch Response**: < 100ms
- **Animation Frame Rate**: 60fps

## Dependencies to Add

```json
{
  "dependencies": {
    "react-spring": "^9.7.0",          // Animations
    "react-swipeable": "^7.0.0",       // Swipe gestures
    "framer-motion": "^11.0.0"         // Alternative animations
  }
}
```

## Estimated Time Remaining

- Icons creation: 1-2 hours
- Mobile layout: 3-4 hours
- Bottom sheet: 2-3 hours
- Gestures: 2-3 hours
- Media Session API: 1-2 hours
- Wake Lock + Haptic: 1 hour
- Testing + Polish: 2-3 hours

**Total**: ~15-20 hours

## Success Criteria

- ✅ PWA installable on iOS/Android
- ✅ Offline support for browsing/playback
- ✅ Native mobile UI (bottom nav, sheets)
- ✅ Touch-optimized (44px+ targets)
- ✅ Media controls in lock screen
- ✅ 60fps animations
- ✅ Lighthouse PWA score > 90

## Current Status

**Phase 6 Progress**: ~15% complete
**Next Steps**:
1. Create PWA icons
2. Implement mobile-responsive layout
3. Add bottom navigation
4. Build bottom sheet player

---

**Last Updated**: 2026-01-29
**Status**: Foundation complete, ready for UI implementation
