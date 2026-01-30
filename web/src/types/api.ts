// API types matching Rust backend structures

export interface Track {
  id: number;
  url: string;
  title?: string;
  artist?: string;
  album?: string;
  genre?: string;
  year?: number;
  track_number?: number;
  tracknum?: number;
  duration?: number;
  secs?: number; // Duration in seconds
  filesize?: number;
  bitrate?: number;
  samplerate?: number;
  channels?: number;
  added_time?: string;
  updated_time?: string;
  has_cover?: boolean;
  lossless?: boolean;
  content_type?: string;
}

export interface Album {
  id: number;
  title: string;
  artist?: string;
  year?: number;
  artwork_url?: string;
  track_count?: number;
}

export interface Artist {
  id: number;
  name: string;
  album_count?: number;
  track_count?: number;
}

export interface Player {
  id: string;
  name: string;
  model: string;
  connected: boolean;
  playing: boolean;
  current_track?: Track;
  position?: number;
  duration?: number;
  volume?: number;
  sync_group_id?: string;
  is_sync_master?: boolean;
}

export interface Playlist {
  id: string;
  name: string;
  track_count: number;
  tracks: Track[];
}

export interface JsonRpcRequest {
  method: string;
  params?: unknown[];
  id?: number | string;
}

export interface JsonRpcResponse<T = unknown> {
  result?: T;
  error?: {
    code: number;
    message: string;
  };
  id?: number | string;
}

export interface NowPlayingState {
  player_id?: string;
  track?: Track;
  position: number;
  duration: number;
  playing: boolean;
  volume: number;
  playlist: Track[];
  playlist_index: number;
}
