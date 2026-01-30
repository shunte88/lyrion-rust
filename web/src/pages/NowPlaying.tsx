import { Stack, Group, Title, Text, Paper, Image, Table, Badge } from '@mantine/core';
import { IconMusic } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';

export function NowPlayingPage() {
  const { nowPlaying, currentPlayer } = useAppStore();

  const coverUrl = nowPlaying.track?.has_cover && nowPlaying.track.id
    ? LyrionAPI.getCoverArtUrl(nowPlaying.track.id)
    : undefined;

  if (!currentPlayer) {
    return (
      <Stack align="center" justify="center" style={{ height: '70vh' }}>
        <IconMusic size={80} stroke={1} color="gray" />
        <Title order={2} c="dimmed">No Player Selected</Title>
        <Text c="dimmed">Select a player to start playing music</Text>
      </Stack>
    );
  }

  if (!nowPlaying.track) {
    return (
      <Stack align="center" justify="center" style={{ height: '70vh' }}>
        <IconMusic size={80} stroke={1} color="gray" />
        <Title order={2} c="dimmed">Nothing Playing</Title>
        <Text c="dimmed">Select a track from your library to start</Text>
      </Stack>
    );
  }

  return (
    <Stack gap="xl">
      {/* Now Playing Card */}
      <Paper shadow="sm" p="xl" radius="md">
        <Group align="flex-start" gap="xl">
          {/* Album Art */}
          <Image
            src={coverUrl}
            fallbackSrc="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='300' height='300'%3E%3Crect width='300' height='300' fill='%23333'/%3E%3Ctext x='50%25' y='50%25' dominant-baseline='middle' text-anchor='middle' fill='%23666' font-size='24'%3ENo Artwork%3C/text%3E%3C/svg%3E"
            w={300}
            h={300}
            radius="md"
          />

          {/* Track Details */}
          <Stack gap="md" style={{ flex: 1 }}>
            <div>
              <Title order={1}>{nowPlaying.track.title}</Title>
              <Text size="xl" c="dimmed" mt="xs">
                {nowPlaying.track.artist || 'Unknown Artist'}
              </Text>
              <Text size="lg" c="dimmed">
                {nowPlaying.track.album || 'Unknown Album'}
              </Text>
            </div>

            <Group gap="md">
              {nowPlaying.track.year && (
                <Badge size="lg" variant="light">
                  {nowPlaying.track.year}
                </Badge>
              )}
              {nowPlaying.track.genre && (
                <Badge size="lg" variant="light" color="blue">
                  {nowPlaying.track.genre}
                </Badge>
              )}
            </Group>

            <Group gap="xl" mt="md">
              {nowPlaying.track.bitrate && (
                <div>
                  <Text size="xs" c="dimmed">Bitrate</Text>
                  <Text size="sm" fw={500}>{nowPlaying.track.bitrate} kbps</Text>
                </div>
              )}
              {nowPlaying.track.samplerate && (
                <div>
                  <Text size="xs" c="dimmed">Sample Rate</Text>
                  <Text size="sm" fw={500}>{nowPlaying.track.samplerate} Hz</Text>
                </div>
              )}
              {nowPlaying.track.channels && (
                <div>
                  <Text size="xs" c="dimmed">Channels</Text>
                  <Text size="sm" fw={500}>{nowPlaying.track.channels}</Text>
                </div>
              )}
            </Group>
          </Stack>
        </Group>
      </Paper>

      {/* Playlist Queue */}
      {nowPlaying.playlist.length > 0 && (
        <Paper shadow="sm" p="md" radius="md">
          <Title order={3} mb="md">Playlist Queue ({nowPlaying.playlist.length} tracks)</Title>

          <Table striped highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th style={{ width: 50 }}>#</Table.Th>
                <Table.Th>Title</Table.Th>
                <Table.Th>Artist</Table.Th>
                <Table.Th>Album</Table.Th>
                <Table.Th style={{ width: 100 }}>Duration</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {nowPlaying.playlist.map((track, index) => (
                <Table.Tr
                  key={track.id}
                  style={{
                    backgroundColor: index === nowPlaying.playlist_index ? 'var(--mantine-color-blue-9)' : undefined,
                  }}
                >
                  <Table.Td>{index + 1}</Table.Td>
                  <Table.Td>{track.title}</Table.Td>
                  <Table.Td>{track.artist || 'Unknown'}</Table.Td>
                  <Table.Td>{track.album || 'Unknown'}</Table.Td>
                  <Table.Td>
                    {track.duration
                      ? `${Math.floor(track.duration / 60)}:${(track.duration % 60).toString().padStart(2, '0')}`
                      : '-'}
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        </Paper>
      )}
    </Stack>
  );
}
