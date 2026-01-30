import { useEffect, useState } from 'react';
import { Stack, Title, Card, Text, Group, ActionIcon, Loader, SimpleGrid, Badge } from '@mantine/core';
import { IconPlayerPlay, IconRefresh } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { notifications } from '@mantine/notifications';

interface Genre {
  name: string;
  trackCount: number;
}

export function GenresPage() {
  const { tracks } = useAppStore();
  const [genres, setGenres] = useState<Genre[]>([]);
  const [loading, setLoading] = useState(false);

  const loadGenres = async () => {
    try {
      setLoading(true);
      const trackData = await LyrionAPI.getTracks(100);
      useAppStore.setState({ tracks: trackData });

      // Group tracks by genre
      const genreMap = new Map<string, number>();
      trackData.forEach((track) => {
        const genre = track.genre || 'Unknown';
        genreMap.set(genre, (genreMap.get(genre) || 0) + 1);
      });

      setGenres(
        Array.from(genreMap.entries())
          .map(([name, trackCount]) => ({ name, trackCount }))
          .sort((a, b) => b.trackCount - a.trackCount) // Sort by track count descending
      );
    } catch (error) {
      notifications.show({
        title: 'Error',
        message: 'Failed to load genres',
        color: 'red',
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadGenres();
  }, []);

  const handlePlayGenre = async (genre: Genre) => {
    const { currentPlayer } = useAppStore.getState();
    if (!currentPlayer) {
      notifications.show({
        title: 'No Player Selected',
        message: 'Please select a player first',
        color: 'yellow',
      });
      return;
    }

    // Find all tracks for this genre
    const genreTracks = tracks.filter((t) => (t.genre || 'Unknown') === genre.name);

    if (genreTracks.length === 0) return;

    try {
      // Clear playlist and add all tracks
      await LyrionAPI.clearPlaylist(currentPlayer.id);
      for (const track of genreTracks.slice(0, 50)) {
        // Limit to 50 tracks
        await LyrionAPI.addTrack(currentPlayer.id, track.id);
      }
      await LyrionAPI.play(currentPlayer.id);

      notifications.show({
        title: 'Playing Genre',
        message: `${genre.name} (${Math.min(50, genreTracks.length)} tracks)`,
      });
    } catch (error) {
      notifications.show({
        title: 'Playback Error',
        message: 'Failed to play genre',
        color: 'red',
      });
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <div>
          <Title order={2}>Genres</Title>
          <Text size="sm" c="dimmed">
            {genres.length} genres
          </Text>
        </div>

        <ActionIcon size="lg" variant="light" onClick={loadGenres} loading={loading}>
          <IconRefresh size={20} />
        </ActionIcon>
      </Group>

      {loading && genres.length === 0 ? (
        <Group justify="center" py="xl">
          <Loader size="lg" />
        </Group>
      ) : (
        <SimpleGrid cols={{ base: 1, sm: 2, md: 3, lg: 4 }} spacing="md">
          {genres.map((genre, idx) => (
            <Card
              key={idx}
              shadow="sm"
              padding="lg"
              radius="md"
              withBorder
              style={{ cursor: 'pointer' }}
            >
              <Stack gap="sm">
                <Group justify="space-between" align="flex-start">
                  <div style={{ flex: 1 }}>
                    <Text fw={500} size="lg" lineClamp={2}>
                      {genre.name}
                    </Text>
                    <Badge size="sm" variant="light" mt="xs">
                      {genre.trackCount} tracks
                    </Badge>
                  </div>
                  <ActionIcon
                    variant="filled"
                    size="lg"
                    radius="xl"
                    onClick={() => handlePlayGenre(genre)}
                  >
                    <IconPlayerPlay size={18} />
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
