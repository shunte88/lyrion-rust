import { useState, useEffect } from 'react';
import { Modal, TextInput, Stack, Text, Group, ActionIcon, ScrollArea, Divider, Badge } from '@mantine/core';
import { IconSearch, IconPlayerPlay, IconDisc, IconMicrophone, IconMusic } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { notifications } from '@mantine/notifications';
import type { Track } from '../types/api';
import { useDebouncedValue } from '@mantine/hooks';

interface SearchModalProps {
  opened: boolean;
  onClose: () => void;
}

interface SearchResults {
  tracks: Track[];
  albums: Map<string, { name: string; artist: string; tracks: Track[] }>;
  artists: Set<string>;
}

export function SearchModal({ opened, onClose }: SearchModalProps) {
  const { tracks } = useAppStore();
  const [query, setQuery] = useState('');
  const [debouncedQuery] = useDebouncedValue(query, 200);
  const [results, setResults] = useState<SearchResults>({
    tracks: [],
    albums: new Map(),
    artists: new Set(),
  });

  useEffect(() => {
    if (!debouncedQuery.trim()) {
      setResults({ tracks: [], albums: new Map(), artists: new Set() });
      return;
    }

    const searchLower = debouncedQuery.toLowerCase();

    // Search tracks
    const matchingTracks = tracks.filter(
      (t) =>
        t.title?.toLowerCase().includes(searchLower) ||
        t.artist?.toLowerCase().includes(searchLower) ||
        t.album?.toLowerCase().includes(searchLower) ||
        t.genre?.toLowerCase().includes(searchLower)
    ).slice(0, 20); // Limit to 20 results

    // Extract albums
    const albumsMap = new Map<string, { name: string; artist: string; tracks: Track[] }>();
    matchingTracks.forEach((track) => {
      const albumKey = `${track.album || 'Unknown'}::${track.artist || 'Unknown'}`;
      if (!albumsMap.has(albumKey)) {
        albumsMap.set(albumKey, {
          name: track.album || 'Unknown Album',
          artist: track.artist || 'Unknown Artist',
          tracks: [],
        });
      }
      albumsMap.get(albumKey)!.tracks.push(track);
    });

    // Extract artists
    const artistsSet = new Set<string>();
    matchingTracks.forEach((track) => {
      if (track.artist) artistsSet.add(track.artist);
    });

    setResults({
      tracks: matchingTracks,
      albums: albumsMap,
      artists: artistsSet,
    });
  }, [debouncedQuery, tracks]);

  const handlePlayTrack = async (track: Track) => {
    const { currentPlayer } = useAppStore.getState();
    if (!currentPlayer) {
      notifications.show({
        title: 'No Player Selected',
        message: 'Please select a player first',
        color: 'yellow',
      });
      return;
    }

    try {
      await LyrionAPI.clearPlaylist(currentPlayer.id);
      await LyrionAPI.addTrack(currentPlayer.id, track.id);
      await LyrionAPI.play(currentPlayer.id);

      notifications.show({
        title: 'Playing Track',
        message: `${track.title || 'Unknown'} - ${track.artist || 'Unknown'}`,
      });
      onClose();
    } catch (error) {
      notifications.show({
        title: 'Playback Error',
        message: 'Failed to play track',
        color: 'red',
      });
    }
  };

  const formatDuration = (seconds?: number) => {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title="Search Music"
      size="xl"
      padding="md"
    >
      <Stack gap="md">
        <TextInput
          placeholder="Search tracks, albums, artists..."
          leftSection={<IconSearch size={16} />}
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          autoFocus
          size="md"
        />

        <ScrollArea h={500}>
          <Stack gap="lg">
            {/* Tracks Section */}
            {results.tracks.length > 0 && (
              <div>
                <Group gap="xs" mb="sm">
                  <IconMusic size={18} />
                  <Text fw={600}>Tracks</Text>
                  <Badge size="sm" variant="light">
                    {results.tracks.length}
                  </Badge>
                </Group>
                <Stack gap="xs">
                  {results.tracks.map((track) => (
                    <Group
                      key={track.id}
                      justify="space-between"
                      p="sm"
                      style={{
                        borderRadius: 'var(--mantine-radius-md)',
                        cursor: 'pointer',
                        '&:hover': {
                          backgroundColor: 'var(--mantine-color-dark-6)',
                        },
                      }}
                    >
                      <div style={{ flex: 1 }}>
                        <Text size="sm" fw={500} lineClamp={1}>
                          {track.title || 'Unknown Title'}
                        </Text>
                        <Text size="xs" c="dimmed" lineClamp={1}>
                          {track.artist || 'Unknown Artist'} • {track.album || 'Unknown Album'}
                        </Text>
                      </div>
                      <Group gap="md">
                        <Text size="xs" c="dimmed">
                          {formatDuration(track.secs)}
                        </Text>
                        <ActionIcon
                          variant="subtle"
                          onClick={() => handlePlayTrack(track)}
                        >
                          <IconPlayerPlay size={18} />
                        </ActionIcon>
                      </Group>
                    </Group>
                  ))}
                </Stack>
              </div>
            )}

            {/* Albums Section */}
            {results.albums.size > 0 && (
              <>
                <Divider />
                <div>
                  <Group gap="xs" mb="sm">
                    <IconDisc size={18} />
                    <Text fw={600}>Albums</Text>
                    <Badge size="sm" variant="light">
                      {results.albums.size}
                    </Badge>
                  </Group>
                  <Stack gap="xs">
                    {Array.from(results.albums.values()).map((album, idx) => (
                      <Group
                        key={idx}
                        p="sm"
                        style={{
                          borderRadius: 'var(--mantine-radius-md)',
                          cursor: 'pointer',
                        }}
                      >
                        <div style={{ flex: 1 }}>
                          <Text size="sm" fw={500}>
                            {album.name}
                          </Text>
                          <Text size="xs" c="dimmed">
                            {album.artist} • {album.tracks.length} tracks
                          </Text>
                        </div>
                      </Group>
                    ))}
                  </Stack>
                </div>
              </>
            )}

            {/* Artists Section */}
            {results.artists.size > 0 && (
              <>
                <Divider />
                <div>
                  <Group gap="xs" mb="sm">
                    <IconMicrophone size={18} />
                    <Text fw={600}>Artists</Text>
                    <Badge size="sm" variant="light">
                      {results.artists.size}
                    </Badge>
                  </Group>
                  <Stack gap="xs">
                    {Array.from(results.artists).map((artist, idx) => (
                      <Group
                        key={idx}
                        p="sm"
                        style={{
                          borderRadius: 'var(--mantine-radius-md)',
                          cursor: 'pointer',
                        }}
                      >
                        <Text size="sm" fw={500}>
                          {artist}
                        </Text>
                      </Group>
                    ))}
                  </Stack>
                </div>
              </>
            )}

            {/* No Results */}
            {debouncedQuery.trim() &&
              results.tracks.length === 0 &&
              results.albums.size === 0 &&
              results.artists.size === 0 && (
                <Text c="dimmed" ta="center" py="xl">
                  No results found for "{debouncedQuery}"
                </Text>
              )}

            {/* Empty State */}
            {!debouncedQuery.trim() && (
              <Text c="dimmed" ta="center" py="xl">
                Start typing to search your music library
              </Text>
            )}
          </Stack>
        </ScrollArea>
      </Stack>
    </Modal>
  );
}
