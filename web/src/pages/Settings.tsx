import { Stack, Title, Text, Card, TextInput, Button, Group, Divider, Badge } from '@mantine/core';
import { IconFolder, IconRefresh, IconDatabase, IconDevices } from '@tabler/icons-react';
import { useState } from 'react';
import { notifications } from '@mantine/notifications';
import { useAppStore } from '../services/store';

export function SettingsPage() {
  const [musicDir, setMusicDir] = useState('/data2/music');
  const [scanning, setScanning] = useState(false);
  const { players } = useAppStore();

  const handleRescan = async () => {
    setScanning(true);
    notifications.show({
      title: 'Scan Started',
      message: 'Scanning music library in background...',
      color: 'blue',
    });

    // Simulate scan (in production, this would trigger backend scan)
    setTimeout(() => {
      setScanning(false);
      notifications.show({
        title: 'Scan Complete',
        message: 'Music library updated successfully',
        color: 'green',
      });
    }, 3000);
  };

  return (
    <Stack gap="lg">
      <div>
        <Title order={2}>Settings</Title>
        <Text size="sm" c="dimmed">
          Configure your Lyrion Music Server
        </Text>
      </div>

      {/* Library Settings */}
      <Card shadow="sm" padding="lg" radius="md" withBorder>
        <Group gap="xs" mb="md">
          <IconFolder size={20} />
          <Text fw={600}>Library Settings</Text>
        </Group>

        <Stack gap="md">
          <TextInput
            label="Music Directory"
            description="Path to your music collection"
            placeholder="/path/to/music"
            value={musicDir}
            onChange={(e) => setMusicDir(e.currentTarget.value)}
            leftSection={<IconFolder size={16} />}
          />

          <div>
            <Text size="sm" fw={500} mb="xs">
              Library Actions
            </Text>
            <Group>
              <Button
                leftSection={<IconRefresh size={16} />}
                onClick={handleRescan}
                loading={scanning}
              >
                Rescan Library
              </Button>
              <Text size="xs" c="dimmed">
                Scan for new and updated tracks
              </Text>
            </Group>
          </div>
        </Stack>
      </Card>

      {/* Server Information */}
      <Card shadow="sm" padding="lg" radius="md" withBorder>
        <Group gap="xs" mb="md">
          <IconDatabase size={20} />
          <Text fw={600}>Server Information</Text>
        </Group>

        <Stack gap="sm">
          <Group justify="space-between">
            <Text size="sm" c="dimmed">
              Server Version
            </Text>
            <Badge variant="light">v0.1.0 (Rust)</Badge>
          </Group>

          <Group justify="space-between">
            <Text size="sm" c="dimmed">
              Database
            </Text>
            <Text size="sm">SQLite</Text>
          </Group>

          <Group justify="space-between">
            <Text size="sm" c="dimmed">
              HTTP Port
            </Text>
            <Text size="sm">9000</Text>
          </Group>

          <Group justify="space-between">
            <Text size="sm" c="dimmed">
              Slimproto Port
            </Text>
            <Text size="sm">3483</Text>
          </Group>
        </Stack>
      </Card>

      {/* Connected Players */}
      <Card shadow="sm" padding="lg" radius="md" withBorder>
        <Group gap="xs" mb="md">
          <IconDevices size={20} />
          <Text fw={600}>Connected Players</Text>
        </Group>

        {players.length === 0 ? (
          <Text size="sm" c="dimmed">
            No players connected
          </Text>
        ) : (
          <Stack gap="sm">
            {players.map((player) => (
              <Group key={player.id} justify="space-between">
                <div>
                  <Text size="sm" fw={500}>
                    {player.name}
                  </Text>
                  <Text size="xs" c="dimmed">
                    ID: {player.id}
                  </Text>
                </div>
                <Badge variant="light" color="green">
                  Connected
                </Badge>
              </Group>
            ))}
          </Stack>
        )}
      </Card>

      <Divider />

      <Text size="xs" c="dimmed" ta="center">
        Lyrion Music Server • Built with Rust + React
      </Text>
    </Stack>
  );
}
