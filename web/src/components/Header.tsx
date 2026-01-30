import { Group, Burger, Title, Select, rem, ActionIcon, Kbd, Tooltip } from '@mantine/core';
import { IconSearch, IconMusic, IconDeviceSpeaker } from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { useEffect, useState } from 'react';
import { LyrionAPI } from '../services/api';
import { SearchModal } from './SearchModal';
import { PlaybackControls } from './PlaybackControls';
import { useHotkeys } from '@mantine/hooks';

export function Header() {
  const { sidebarOpen, toggleSidebar, players, currentPlayer, setCurrentPlayer } = useAppStore();
  const [searchOpened, setSearchOpened] = useState(false);

  // Keyboard shortcut: Ctrl/Cmd + K to open search
  useHotkeys([['mod+K', () => setSearchOpened(true)]]);

  useEffect(() => {
    // Load players on mount
    LyrionAPI.getPlayers().then((playerList) => {
      useAppStore.setState({ players: playerList });
      // Auto-select first player if none selected
      if (!currentPlayer && playerList.length > 0) {
        setCurrentPlayer(playerList[0]);
      }
    });
  }, []);

  return (
    <>
      <Group h="100%" px="md" justify="space-between">
        <Group>
          <Burger opened={sidebarOpen} onClick={toggleSidebar} size="sm" />
          <Group gap="xs">
            <IconMusic size={28} />
            <Title order={3}>Lyrion</Title>
          </Group>
        </Group>

        <Group gap="md">
          <PlaybackControls />

          <Select
            placeholder="Select player"
            leftSection={<IconDeviceSpeaker style={{ width: rem(16), height: rem(16) }} />}
            data={players.map((p) => ({
              value: p.mac || p.uuid || p.id || '',
              label: p.name || p.mac || 'Unknown Player'
            }))}
            value={currentPlayer?.mac || currentPlayer?.uuid || currentPlayer?.id}
            onChange={(value) => {
              const player = players.find((p) => (p.mac || p.uuid || p.id) === value);
              if (player) {
                // Use MAC as the primary ID (UUID can be all zeros for some players)
                setCurrentPlayer({ ...player, id: player.mac || player.uuid || player.id });
              }
            }}
            style={{ width: 200 }}
          />

          <Tooltip label={<Group gap={4}>Search <Kbd size="xs">⌘</Kbd><Kbd size="xs">K</Kbd></Group>}>
            <ActionIcon
              variant="subtle"
              size="lg"
              onClick={() => setSearchOpened(true)}
            >
              <IconSearch size={20} />
            </ActionIcon>
          </Tooltip>
        </Group>
      </Group>

      <SearchModal opened={searchOpened} onClose={() => setSearchOpened(false)} />
    </>
  );
}
