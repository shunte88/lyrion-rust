import { Group, ActionIcon, Text, Stack, Progress, Slider } from '@mantine/core';
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerSkipBack,
  IconPlayerSkipForward,
  IconVolume,
} from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';

export function PlayerBar() {
  const { nowPlaying, currentPlayer } = useAppStore();

  const handlePlay = async () => {
    if (!currentPlayer) return;
    if (nowPlaying.playing) {
      await LyrionAPI.pause(currentPlayer.id);
    } else {
      await LyrionAPI.play(currentPlayer.id);
    }
  };

  const handlePrevious = async () => {
    if (!currentPlayer) return;
    await LyrionAPI.previous(currentPlayer.id);
  };

  const handleNext = async () => {
    if (!currentPlayer) return;
    await LyrionAPI.next(currentPlayer.id);
  };

  const handleVolumeChange = async (value: number) => {
    if (!currentPlayer) return;
    await LyrionAPI.setVolume(currentPlayer.id, value);
  };

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const progress = nowPlaying.duration > 0
    ? (nowPlaying.position / nowPlaying.duration) * 100
    : 0;

  return (
    <Stack gap="xs" p="md" style={{ height: '100%' }}>
      {/* Progress bar */}
      <Progress value={progress} size="xs" />

      <Group justify="space-between" style={{ flex: 1 }}>
        {/* Track info */}
        <Stack gap={0} style={{ flex: 1 }}>
          <Text size="sm" fw={500} lineClamp={1}>
            {nowPlaying.track?.title || 'No track playing'}
          </Text>
          <Text size="xs" c="dimmed" lineClamp={1}>
            {nowPlaying.track?.artist || 'Unknown Artist'}
          </Text>
        </Stack>

        {/* Playback controls */}
        <Group gap="md">
          <ActionIcon
            size="lg"
            variant="subtle"
            onClick={handlePrevious}
            disabled={!currentPlayer}
          >
            <IconPlayerSkipBack size={24} />
          </ActionIcon>

          <ActionIcon
            size="xl"
            variant="filled"
            onClick={handlePlay}
            disabled={!currentPlayer}
          >
            {nowPlaying.playing ? (
              <IconPlayerPause size={28} />
            ) : (
              <IconPlayerPlay size={28} />
            )}
          </ActionIcon>

          <ActionIcon
            size="lg"
            variant="subtle"
            onClick={handleNext}
            disabled={!currentPlayer}
          >
            <IconPlayerSkipForward size={24} />
          </ActionIcon>
        </Group>

        {/* Volume and time */}
        <Group gap="md" style={{ width: 250 }} justify="flex-end">
          <Text size="xs" c="dimmed">
            {formatTime(nowPlaying.position)} / {formatTime(nowPlaying.duration)}
          </Text>

          <Group gap="xs" style={{ width: 120 }}>
            <IconVolume size={20} />
            <Slider
              value={nowPlaying.volume}
              onChange={handleVolumeChange}
              min={0}
              max={100}
              style={{ flex: 1 }}
              disabled={!currentPlayer}
            />
          </Group>
        </Group>
      </Group>
    </Stack>
  );
}
