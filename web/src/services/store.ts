// Global state management with Zustand

import { create } from 'zustand';
import type { Track, Player, NowPlayingState } from '../types/api';

interface AppState {
  // Current player
  currentPlayer: Player | null;
  setCurrentPlayer: (player: Player | null) => void;

  // Players list
  players: Player[];
  setPlayers: (players: Player[]) => void;

  // Now playing
  nowPlaying: NowPlayingState;
  setNowPlaying: (state: Partial<NowPlayingState>) => void;

  // Library
  tracks: Track[];
  setTracks: (tracks: Track[]) => void;
  searchQuery: string;
  setSearchQuery: (query: string) => void;

  // UI state
  sidebarOpen: boolean;
  toggleSidebar: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Current player
  currentPlayer: null,
  setCurrentPlayer: (player) => set({ currentPlayer: player }),

  // Players
  players: [],
  setPlayers: (players) => set({ players }),

  // Now playing
  nowPlaying: {
    position: 0,
    duration: 0,
    playing: false,
    volume: 50,
    playlist: [],
    playlist_index: 0,
    shuffle: 0,
    repeat: 0,
  },
  setNowPlaying: (state) =>
    set((prev) => ({
      nowPlaying: { ...prev.nowPlaying, ...state },
    })),

  // Library
  tracks: [],
  setTracks: (tracks) => set({ tracks }),
  searchQuery: '',
  setSearchQuery: (query) => set({ searchQuery: query }),

  // UI
  sidebarOpen: true,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
}));
