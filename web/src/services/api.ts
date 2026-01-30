// API service for Lyrion Music Server backend

import type { Track, Player, JsonRpcRequest, JsonRpcResponse } from '../types/api';

const API_BASE = '/api/v1';
const JSONRPC_ENDPOINT = '/jsonrpc.js';

export class LyrionAPI {
  // Tracks
  static async getTracks(limit = 100, offset = 0): Promise<Track[]> {
    const response = await fetch(`${API_BASE}/tracks?limit=${limit}&offset=${offset}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch tracks: ${response.statusText}`);
    }
    return response.json();
  }

  static async searchTracks(query: string, limit = 50): Promise<Track[]> {
    const response = await fetch(`${API_BASE}/tracks/search?q=${encodeURIComponent(query)}&limit=${limit}`);
    if (!response.ok) {
      throw new Error(`Failed to search tracks: ${response.statusText}`);
    }
    return response.json();
  }

  // Players
  static async getPlayers(): Promise<Player[]> {
    const response = await fetch(`${API_BASE}/players`);
    if (!response.ok) {
      throw new Error(`Failed to fetch players: ${response.statusText}`);
    }
    return response.json();
  }

  // JSON-RPC
  static async jsonrpc<T = unknown>(
    playerId: string,
    command: string[],
    id?: number | string
  ): Promise<JsonRpcResponse<T>> {
    const request: JsonRpcRequest = {
      method: 'slim.request',
      params: [playerId, command],
      id: id ?? Date.now(),
    };

    const response = await fetch(JSONRPC_ENDPOINT, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request),
    });

    if (!response.ok) {
      throw new Error(`JSON-RPC failed: ${response.statusText}`);
    }

    return response.json();
  }

  // Player commands
  static async play(playerId: string) {
    return this.jsonrpc(playerId, ['play']);
  }

  static async pause(playerId: string) {
    return this.jsonrpc(playerId, ['pause']);
  }

  static async stop(playerId: string) {
    return this.jsonrpc(playerId, ['stop']);
  }

  static async next(playerId: string) {
    return this.jsonrpc(playerId, ['playlist', 'index', '+1']);
  }

  static async previous(playerId: string) {
    return this.jsonrpc(playerId, ['playlist', 'index', '-1']);
  }

  static async setVolume(playerId: string, volume: number) {
    return this.jsonrpc(playerId, ['mixer', 'volume', volume.toString()]);
  }

  static async getStatus(playerId: string) {
    return this.jsonrpc(playerId, ['status', '-', '1', 'tags:adlty']);
  }

  // Sync commands
  static async syncPlayers(playerId: string, masterPlayerId: string) {
    return this.jsonrpc(playerId, ['sync', masterPlayerId]);
  }

  static async unsyncPlayer(playerId: string) {
    return this.jsonrpc(playerId, ['sync', '-']);
  }

  static async getSyncGroup(playerId: string) {
    return this.jsonrpc(playerId, ['syncgroupid']);
  }

  // Playlist commands
  static async loadTrack(playerId: string, trackId: number) {
    return this.jsonrpc(playerId, ['playlist', 'play', `track_id:${trackId}`]);
  }

  static async addTrack(playerId: string, trackId: number) {
    return this.jsonrpc(playerId, ['playlist', 'add', `track_id:${trackId}`]);
  }

  static async clearPlaylist(playerId: string) {
    return this.jsonrpc(playerId, ['playlist', 'clear']);
  }

  // Streaming URL
  static getStreamUrl(trackId: number, format?: string): string {
    const params = format ? `?format=${format}` : '';
    return `/stream/${trackId}${params}`;
  }

  // Cover art URL
  static getCoverArtUrl(trackId: number): string {
    return `/api/v1/cover/${trackId}`;
  }
}
