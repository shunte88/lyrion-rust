import { useEffect, useState } from 'react';
import { Stack, Title, Table, ActionIcon, Group, Loader, Text, Badge } from '@mantine/core';
import { IconPlayerPlay, IconRefresh } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { playTracks } from '../services/playerUtils';
import { notifications } from '@mantine/notifications';

interface Artist {
  name: string;
  trackCount: number;
  albumCount: number;
}

export function ArtistsPage() {
  const { tracks } = useAppStore();
  const [artists, setArtists] = useState<Artist[]>([]);
  const [loading, setLoading] = useState(false);

  const loadArtists = async () => {
    try {
      setLoading(true);
      const trackData = await LyrionAPI.getTracks(100);
      useAppStore.setState({ tracks: trackData });

      // Group by artist
      const artistMap = new Map<string, { albums: Set<string>; tracks: number }>();
      trackData.forEach((track) => {
        const artistName = track.artist || 'Unknown Artist';
        if (artistMap.has(artistName)) {
          const artist = artistMap.get(artistName)!;
          artist.tracks++;
          if (track.album) {
            artist.albums.add(track.album);
          }
        } else {
          artistMap.set(artistName, {
            albums: new Set(track.album ? [track.album] : []),
            tracks: 1,
          });
        }
      });

      const artistList: Artist[] = Array.from(artistMap.entries()).map(([name, data]) => ({
        name,
        trackCount: data.tracks,
        albumCount: data.albums.size,
      }));

      setArtists(artistList.sort((a, b) => a.name.localeCompare(b.name)));
    } catch (error) {
      notifications.show({
        title: 'Error',
        message: 'Failed to load artists',
        color: 'red',
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadArtists();
  }, []);

  const handlePlayArtist = async (artist: Artist) => {
    const { currentPlayer } = useAppStore.getState();
    if (!currentPlayer) {
      notifications.show({
        title: 'No Player Selected',
        message: 'Please select a player first',
        color: 'yellow',
      });
      return;
    }

    // Find all tracks for this artist
    const artistTracks = tracks.filter((t) => t.artist === artist.name);

    if (artistTracks.length === 0) return;

    try {
      // Play all tracks by this artist
      const trackIds = artistTracks.map((t) => t.id);
      await playTracks(currentPlayer.mac || currentPlayer.id || currentPlayer.uuid || '', trackIds);

      notifications.show({
        title: 'Playing Artist',
        message: artist.name,
      });
    } catch (error) {
      notifications.show({
        title: 'Playback Error',
        message: 'Failed to play artist',
        color: 'red',
      });
    }
  };

  return (
    <Stack gap="md">
      <Group justify="space-between">
        <div>
          <Title order={2}>Artists</Title>
          <Text size="sm" c="dimmed">
            {artists.length} artists
          </Text>
        </div>

        <ActionIcon size="lg" variant="light" onClick={loadArtists} loading={loading}>
          <IconRefresh size={20} />
        </ActionIcon>
      </Group>

      {loading && artists.length === 0 ? (
        <Group justify="center" py="xl">
          <Loader size="lg" />
        </Group>
      ) : (
        <Table striped highlightOnHover>
          <Table.Thead>
            <Table.Tr>
              <Table.Th style={{ width: 50 }}></Table.Th>
              <Table.Th>Artist</Table.Th>
              <Table.Th style={{ width: 150 }}>Albums</Table.Th>
              <Table.Th style={{ width: 150 }}>Tracks</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {artists.map((artist, idx) => (
              <Table.Tr key={idx}>
                <Table.Td>
                  <ActionIcon
                    size="sm"
                    variant="subtle"
                    onClick={() => handlePlayArtist(artist)}
                  >
                    <IconPlayerPlay size={16} />
                  </ActionIcon>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" fw={500} lineClamp={1}>
                    {artist.name}
                  </Text>
                </Table.Td>
                <Table.Td>
                  <Badge size="sm" variant="light">
                    {artist.albumCount} {artist.albumCount === 1 ? 'album' : 'albums'}
                  </Badge>
                </Table.Td>
                <Table.Td>
                  <Text size="sm" c="dimmed">
                    {artist.trackCount} {artist.trackCount === 1 ? 'track' : 'tracks'}
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
