import { useState, useEffect } from 'react';
import { Modal, TextInput, Stack, Text, Group, ActionIcon, ScrollArea, Divider, Badge, Loader, Center, Alert } from '@mantine/core';
import { IconSearch, IconPlayerPlay, IconDisc, IconMicrophone, IconMusic } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { playTrack } from '../services/playerUtils';
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
  const [query, setQuery] = useState('');
  const [debouncedQuery] = useDebouncedValue(query, 200);
  const [results, setResults] = useState<SearchResults>({
    tracks: [],
    albums: new Map(),
    artists: new Set(),
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!debouncedQuery.trim() || debouncedQuery.length < 2) {
      setResults({ tracks: [], albums: new Map(), artists: new Set() });
      setLoading(false);
      return;
    }

    const controller = new AbortController();

    const searchTracks = async () => {
      setLoading(true);
      setError(null);
      try {
        // Call backend API
        const matchingTracks = await LyrionAPI.searchTracks(debouncedQuery, 100);

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
      } catch (err: any) {
        if (err.name !== 'AbortError') {
          setError('Search failed. Please try again.');
          console.error('Search error:', err);
        }
      } finally {
        setLoading(false);
      }
    };

    searchTracks();

    return () => controller.abort();
  }, [debouncedQuery]);

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
      await playTrack(currentPlayer.mac || currentPlayer.id || currentPlayer.uuid || '', track.id);

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

        {/* Error Alert */}
        {error && (
          <Alert color="red" title="Error">
            {error}
          </Alert>
        )}

        <ScrollArea h={500}>
          <Stack gap="lg">
            {/* Loading State */}
            {loading && (
              <Center py="xl">
                <Loader size="lg" />
              </Center>
            )}

            {/* Minimum Query Length Hint */}
            {debouncedQuery.trim().length > 0 && debouncedQuery.trim().length < 2 && !loading && (
              <Center py="xl">
                <Text c="dimmed" ta="center">
                  Type at least 2 characters to search
                </Text>
              </Center>
            )}

            {/* Tracks Section */}
            {!loading && results.tracks.length > 0 && (
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
            {!loading && results.albums.size > 0 && (
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
            {!loading && results.artists.size > 0 && (
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
            {!loading &&
              debouncedQuery.trim().length >= 2 &&
              results.tracks.length === 0 &&
              results.albums.size === 0 &&
              results.artists.size === 0 && (
                <Center py="xl">
                  <Stack align="center" gap="xs">
                    <IconSearch size={48} stroke={1.5} style={{ opacity: 0.3 }} />
                    <Text c="dimmed" ta="center">
                      No results found for "{debouncedQuery}"
                    </Text>
                  </Stack>
                </Center>
              )}

            {/* Empty State */}
            {!loading && !debouncedQuery.trim() && (
              <Center py="xl">
                <Text c="dimmed" ta="center">
                  Start typing to search your music library
                </Text>
              </Center>
            )}
          </Stack>
        </ScrollArea>
      </Stack>
    </Modal>
  );
}
