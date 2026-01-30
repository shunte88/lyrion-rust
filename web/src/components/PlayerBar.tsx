import { Group, ActionIcon, Text, Stack, Slider } from '@mantine/core';
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerStop,
  IconPlayerSkipBack,
  IconPlayerSkipForward,
  IconVolume,
  IconArrowsShuffle,
  IconRepeat,
  IconRepeatOnce,
} from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';

export function PlayerBar() {
  const { nowPlaying, currentPlayer } = useAppStore();
  const playerId = currentPlayer?.mac || currentPlayer?.id || currentPlayer?.uuid || '';

  const handlePlay = async () => {
    if (!currentPlayer || !playerId) return;
    if (nowPlaying.playing) {
      await LyrionAPI.pause(playerId);
    } else {
      await LyrionAPI.play(playerId);
    }
  };

  const handlePrevious = async () => {
    if (!currentPlayer || !playerId) return;
    await LyrionAPI.previous(playerId);
  };

  const handleNext = async () => {
    if (!currentPlayer || !playerId) return;
    await LyrionAPI.next(playerId);
  };

  const handleStop = async () => {
    if (!currentPlayer || !playerId) return;
    await LyrionAPI.stop(playerId);
  };

  const handleVolumeChange = async (value: number) => {
    if (!currentPlayer || !playerId) return;
    await LyrionAPI.setVolume(playerId, value);
  };

  const handleShuffle = async () => {
    if (!currentPlayer || !playerId) return;
    // Cycle through: 0 (off) -> 1 (songs) -> 2 (albums) -> 0
    const nextMode = ((nowPlaying.shuffle + 1) % 3) as 0 | 1 | 2;
    await LyrionAPI.setShuffle(playerId, nextMode);
    useAppStore.setState((state) => ({
      nowPlaying: { ...state.nowPlaying, shuffle: nextMode },
    }));
  };

  const handleRepeat = async () => {
    if (!currentPlayer || !playerId) return;
    // Cycle through: 0 (off) -> 2 (playlist) -> 1 (song) -> 0
    const nextMode = nowPlaying.repeat === 0 ? 2 : nowPlaying.repeat === 2 ? 1 : 0;
    await LyrionAPI.setRepeat(playerId, nextMode as 0 | 1 | 2);
    useAppStore.setState((state) => ({
      nowPlaying: { ...state.nowPlaying, repeat: nextMode },
    }));
  };

  const handleSeek = async (value: number) => {
    if (!currentPlayer || !playerId || !nowPlaying.duration) return;
    const position = (value / 100) * nowPlaying.duration;
    await LyrionAPI.seek(playerId, position);
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
      {/* Seek bar */}
      <Slider
        value={progress}
        onChange={handleSeek}
        min={0}
        max={100}
        size="xs"
        disabled={!currentPlayer || !nowPlaying.duration}
        label={(val) => formatTime((val / 100) * (nowPlaying.duration || 0))}
        style={{ cursor: currentPlayer && nowPlaying.duration ? 'pointer' : 'default' }}
      />

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
            onClick={handleStop}
            disabled={!currentPlayer}
          >
            <IconPlayerStop size={24} />
          </ActionIcon>

          <ActionIcon
            size="lg"
            variant="subtle"
            onClick={handleNext}
            disabled={!currentPlayer}
          >
            <IconPlayerSkipForward size={24} />
          </ActionIcon>

          {/* Shuffle button */}
          <ActionIcon
            size="lg"
            variant="subtle"
            onClick={handleShuffle}
            disabled={!currentPlayer}
            color={nowPlaying.shuffle > 0 ? 'blue' : 'gray'}
          >
            <IconArrowsShuffle size={20} />
          </ActionIcon>

          {/* Repeat button */}
          <ActionIcon
            size="lg"
            variant="subtle"
            onClick={handleRepeat}
            disabled={!currentPlayer}
            color={nowPlaying.repeat > 0 ? 'blue' : 'gray'}
          >
            {nowPlaying.repeat === 1 ? (
              <IconRepeatOnce size={20} />
            ) : (
              <IconRepeat size={20} />
            )}
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
