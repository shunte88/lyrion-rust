import { useState } from 'react';
import { MantineProvider, AppShell } from '@mantine/core';
import { useMediaQuery } from '@mantine/hooks';
import { Notifications } from '@mantine/notifications';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Header } from './components/Header';
import { Sidebar } from './components/Sidebar';
import { MobileNav } from './components/MobileNav';
import { PlayerBar } from './components/PlayerBar';
import { NowPlayingSheet } from './components/NowPlayingSheet';
import { NowPlayingPage } from './pages/NowPlaying';
import { LibraryPage } from './pages/Library';
import { AlbumsPage } from './pages/Albums';
import { ArtistsPage } from './pages/Artists';
import { TracksPage } from './pages/Tracks';
import { GenresPage } from './pages/Genres';
import { SettingsPage } from './pages/Settings';
import { useAppStore } from './services/store';
import { useWebSocket } from './hooks/useWebSocket';
import { useMediaSession } from './hooks/useMediaSession';
import { useWakeLock } from './hooks/useWakeLock';
import { lyrionTheme } from './styles/theme';

import '@mantine/core/styles.css';
import '@mantine/notifications/styles.css';

export function App() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const isMobile = useMediaQuery('(max-width: 768px)');
  const [sheetOpen, setSheetOpen] = useState(true);

  // Initialize hooks
  useWebSocket();
  useMediaSession();
  useWakeLock();

  return (
    <MantineProvider theme={lyrionTheme} defaultColorScheme="dark">
      <Notifications position="top-right" />
      <BrowserRouter>
        <AppShell
          header={{ height: 60 }}
          navbar={
            isMobile
              ? undefined
              : {
                  width: 250,
                  breakpoint: 'sm',
                  collapsed: { mobile: !sidebarOpen, desktop: !sidebarOpen },
                }
          }
          footer={{ height: isMobile ? 60 : 100 }} // Only mobile nav on mobile
          padding="md"
        >
          <AppShell.Header>
            <Header />
          </AppShell.Header>

          {!isMobile && (
            <AppShell.Navbar>
              <Sidebar />
            </AppShell.Navbar>
          )}

          <AppShell.Main>
            <Routes>
              <Route path="/" element={<Navigate to="/now-playing" replace />} />
              <Route path="/now-playing" element={<NowPlayingPage />} />
              <Route path="/library" element={<LibraryPage />} />
              <Route path="/tracks" element={<TracksPage />} />
              <Route path="/albums" element={<AlbumsPage />} />
              <Route path="/artists" element={<ArtistsPage />} />
              <Route path="/genres" element={<GenresPage />} />
              <Route path="/settings" element={<SettingsPage />} />
            </Routes>
          </AppShell.Main>

          <AppShell.Footer>
            {!isMobile && <PlayerBar />}
            {isMobile && <MobileNav />}
          </AppShell.Footer>
        </AppShell>

        {/* Mobile Now Playing Sheet */}
        {isMobile && (
          <NowPlayingSheet
            open={sheetOpen}
            onClose={() => setSheetOpen(false)}
          />
        )}
      </BrowserRouter>
    </MantineProvider>
  );
}
