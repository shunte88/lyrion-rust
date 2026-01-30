import { useEffect, useState } from 'react';
import { Stack, Title, SimpleGrid, Card, Image, Text, Group, ActionIcon, Loader, AspectRatio } from '@mantine/core';
import { IconPlayerPlay, IconRefresh } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { notifications } from '@mantine/notifications';
import type { Track } from '../types/api';

interface Album {
  name: string;
  artist: string;
  trackCount: number;
  year?: number;
  coverTrack?: Track;
}

export function AlbumsPage() {
  const { tracks } = useAppStore();
  const [albums, setAlbums] = useState<Album[]>([]);
  const [loading, setLoading] = useState(false);

  const loadAlbums = async () => {
    try {
      setLoading(true);
      const trackData = await LyrionAPI.getTracks(100);
      useAppStore.setState({ tracks: trackData });

      // Group tracks by album
      const albumMap = new Map<string, Album>();
      trackData.forEach((track) => {
        const albumKey = `${track.album || 'Unknown'}::${track.artist || 'Unknown'}`;
        if (albumMap.has(albumKey)) {
          const album = albumMap.get(albumKey)!;
          album.trackCount++;
          // Keep track with cover art if available
          if (track.has_cover && !album.coverTrack) {
            album.coverTrack = track;
          }
        } else {
          albumMap.set(albumKey, {
            name: track.album || 'Unknown Album',
            artist: track.artist || 'Unknown Artist',
            trackCount: 1,
            year: track.year,
            coverTrack: track.has_cover ? track : undefined,
          });
        }
      });

      setAlbums(Array.from(albumMap.values()).sort((a, b) => a.name.localeCompare(b.name)));
    } catch (error) {
      notifications.show({
        title: 'Error',
        message: 'Failed to load albums',
        color: 'red',
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAlbums();
  }, []);

  const handlePlayAlbum = async (album: Album) => {
    const { currentPlayer } = useAppStore.getState();
    if (!currentPlayer) {
      notifications.show({
        title: 'No Player Selected',
        message: 'Please select a player first',
        color: 'yellow',
      });
      return;
    }

    // Find all tracks for this album
    const albumTracks = tracks.filter(
      (t) => t.album === album.name && t.artist === album.artist
    );

    if (albumTracks.length === 0) return;

    try {
      // Clear playlist and add all tracks
      await LyrionAPI.clearPlaylist(currentPlayer.id);
      for (const track of albumTracks) {
        await LyrionAPI.addTrack(currentPlayer.id, track.id);
      }
      await LyrionAPI.play(currentPlayer.id);

      notifications.show({
        title: 'Playing Album',
        message: `${album.name} - ${album.artist}`,
      });
    } catch (error) {
      notifications.show({
        title: 'Playback Error',
        message: 'Failed to play album',
        color: 'red',
      });
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <div>
          <Title order={2}>Albums</Title>
          <Text size="sm" c="dimmed">
            {albums.length} albums
          </Text>
        </div>

        <ActionIcon size="lg" variant="light" onClick={loadAlbums} loading={loading}>
          <IconRefresh size={20} />
        </ActionIcon>
      </Group>

      {loading && albums.length === 0 ? (
        <Group justify="center" py="xl">
          <Loader size="lg" />
        </Group>
      ) : (
        <SimpleGrid cols={{ base: 2, sm: 3, md: 4, lg: 5, xl: 6 }} spacing="lg">
          {albums.map((album, idx) => (
            <Card key={idx} shadow="sm" padding="md" radius="md" withBorder>
              <Card.Section>
                <AspectRatio ratio={1} maw="100%">
                  <Image
                    src={album.coverTrack ? LyrionAPI.getCoverArtUrl(album.coverTrack.id) : undefined}
                    fallbackSrc="https://placehold.co/400x400/1a1b1e/white?text=No+Cover"
                    alt={album.name}
                    fit="cover"
                  />
                </AspectRatio>
              </Card.Section>

              <Stack gap="xs" mt="md" mb="xs">
                <Text fw={500} lineClamp={2} size="sm">
                  {album.name}
                </Text>
                <Text size="xs" c="dimmed" lineClamp={1}>
                  {album.artist}
                </Text>
                <Group justify="space-between" align="center">
                  <Text size="xs" c="dimmed">
                    {album.trackCount} tracks
                    {album.year && ` • ${album.year}`}
                  </Text>
                  <ActionIcon
                    variant="filled"
                    size="md"
                    radius="xl"
                    onClick={() => handlePlayAlbum(album)}
                  >
                    <IconPlayerPlay size={16} />
                  </ActionIcon>
                </Group>
              </Stack>
            </Card>
          ))}
        </SimpleGrid>
      )}
    </Stack>
  );
}
