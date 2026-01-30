import { useEffect, useState } from 'react';
import { Stack, Title, Table, Text, Group, ActionIcon, Loader, Badge } from '@mantine/core';
import { useMediaQuery } from '@mantine/hooks';
import { IconPlayerPlay, IconRefresh } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { notifications } from '@mantine/notifications';
import { TrackCard } from '../components/TrackCard';
import type { Track } from '../types/api';

export function TracksPage() {
  const { tracks } = useAppStore();
  const [loading, setLoading] = useState(false);
  const isMobile = useMediaQuery('(max-width: 768px)');

  const loadTracks = async () => {
    try {
      setLoading(true);
      const trackData = await LyrionAPI.getTracks(100);
      useAppStore.setState({ tracks: trackData });
    } catch (error) {
      notifications.show({
        title: 'Error',
        message: 'Failed to load tracks',
        color: 'red',
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadTracks();
  }, []);

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
    <Stack gap="md">
      <Group justify="space-between">
        <div>
          <Title order={2}>Tracks</Title>
          <Text size="sm" c="dimmed">
            {tracks.length} tracks
          </Text>
        </div>

        <ActionIcon size="lg" variant="light" onClick={loadTracks} loading={loading}>
          <IconRefresh size={20} />
        </ActionIcon>
      </Group>

      {loading && tracks.length === 0 ? (
        <Group justify="center" py="xl">
          <Loader size="lg" />
        </Group>
      ) : isMobile ? (
        <Stack gap="sm">
          {tracks.map((track, idx) => (
            <TrackCard
              key={track.id}
              track={track}
              trackNumber={track.tracknum || idx + 1}
              onPlay={handlePlayTrack}
            />
          ))}
        </Stack>
      ) : (
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th style={{ width: '40px' }}>#</Table.Th>
              <Table.Th>Title</Table.Th>
              <Table.Th>Artist</Table.Th>
              <Table.Th>Album</Table.Th>
              <Table.Th style={{ width: '80px' }}>Year</Table.Th>
              <Table.Th style={{ width: '80px' }}>Duration</Table.Th>
              <Table.Th style={{ width: '100px' }}>Format</Table.Th>
              <Table.Th style={{ width: '60px' }}></Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {tracks.map((track, idx) => (
              <Table.Tr key={track.id} style={{ cursor: 'pointer' }}>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {track.tracknum || idx + 1}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" fw={500} lineClamp={1}>
                    {track.title || 'Unknown Title'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed" lineClamp={1}>
                    {track.artist || 'Unknown Artist'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed" lineClamp={1}>
                    {track.album || 'Unknown Album'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {track.year || '—'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {formatDuration(track.secs)}
                  </Text>
                </Table.Td>
                <Table.Td>
                  {track.lossless ? (
                    <Badge size="xs" variant="light" color="green">
                      FLAC
                    </Badge>
                  ) : (
                    <Badge size="xs" variant="light" color="blue">
                      {track.content_type?.toUpperCase() || 'MP3'}
                    </Badge>
                  )}
                </Table.Td>
                <Table.Td>
                  <ActionIcon
                    variant="subtle"
                    size="sm"
                    onClick={() => handlePlayTrack(track)}
                  >
                    <IconPlayerPlay size={16} />
                  </ActionIcon>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}
    </Stack>
  );
}
