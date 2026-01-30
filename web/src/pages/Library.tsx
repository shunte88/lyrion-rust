import { useEffect, useState } from 'react';
import { Stack, Title, Table, ActionIcon, Group, Loader, Text, Badge } from '@mantine/core';
import { IconPlayerPlay, IconRefresh } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { notifications } from '@mantine/notifications';
import type { Track } from '../types/api';

export function LibraryPage() {
  const { tracks, setTracks, searchQuery } = useAppStore();
  const [loading, setLoading] = useState(false);

  const loadTracks = async () => {
    try {
      setLoading(true);
      const data = await LyrionAPI.getTracks(500);
      setTracks(data);
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

  const filteredTracks = searchQuery
    ? tracks.filter((track) => {
        const query = searchQuery.toLowerCase();
        return (
          track.title?.toLowerCase().includes(query) ||
          track.artist?.toLowerCase().includes(query) ||
          track.album?.toLowerCase().includes(query)
        );
      })
    : tracks;

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
      // Load track into player's queue and play
      await LyrionAPI.loadTrack(currentPlayer.id, track.id);
      await LyrionAPI.play(currentPlayer.id);

      notifications.show({
        title: 'Playing',
        message: `${track.title || 'Unknown'} - ${track.artist || 'Unknown'}`,
      });
    } catch (error) {
      notifications.show({
        title: 'Playback Error',
        message: 'Failed to start playback',
        color: 'red',
      });
    }
  };

  const formatDuration = (seconds?: number) => {
    if (!seconds) return '-';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const formatFilesize = (bytes?: number) => {
    if (!bytes) return '-';
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <div>
          <Title order={2}>Library</Title>
          <Text size="sm" c="dimmed">
            {filteredTracks.length} tracks
            {searchQuery && ` (filtered from ${tracks.length})`}
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
      ) : (
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th style={{ width: 50 }}></Table.Th>
              <Table.Th>Title</Table.Th>
              <Table.Th>Artist</Table.Th>
              <Table.Th>Album</Table.Th>
              <Table.Th>Genre</Table.Th>
              <Table.Th style={{ width: 80 }}>Year</Table.Th>
              <Table.Th style={{ width: 100 }}>Duration</Table.Th>
              <Table.Th style={{ width: 100 }}>Size</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {filteredTracks.map((track) => (
              <Table.Tr key={track.id}>
                <Table.Td>
                  <ActionIcon
                    size="sm"
                    variant="subtle"
                    onClick={() => handlePlayTrack(track)}
                  >
                    <IconPlayerPlay size={16} />
                  </ActionIcon>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" fw={500} lineClamp={1}>
                    {track.title || 'Unknown Title'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed" lineClamp={1}>
                    {track.artist || 'Unknown'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed" lineClamp={1}>
                    {track.album || 'Unknown'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  {track.genre && (
                    <Badge size="sm" variant="light">
                      {track.genre}
                    </Badge>
                  )}
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {track.year || '-'}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {formatDuration(track.duration)}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {formatFilesize(track.filesize)}
                  </Text>
                </Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}
    </Stack>
  );
}
