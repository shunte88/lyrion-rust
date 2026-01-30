import { Group, ActionIcon, Slider, Text } from '@mantine/core';
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

export function PlaybackControls() {
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
    console.log('[PlaybackControls] Setting volume to:', value, 'for player:', playerId);
    try {
      const response = await LyrionAPI.setVolume(playerId, value);
      console.log('[PlaybackControls] Volume response:', response);
      // Update local state immediately for responsiveness
      useAppStore.setState((state) => ({
        nowPlaying: { ...state.nowPlaying, volume: value },
      }));
    } catch (error) {
      console.error('[PlaybackControls] Volume change failed:', error);
    }
  };

  const handleShuffle = async () => {
    if (!currentPlayer || !playerId) return;
    const nextMode = ((nowPlaying.shuffle + 1) % 3) as 0 | 1 | 2;
    await LyrionAPI.setShuffle(playerId, nextMode);
    useAppStore.setState((state) => ({
      nowPlaying: { ...state.nowPlaying, shuffle: nextMode },
    }));
  };

  const handleRepeat = async () => {
    if (!currentPlayer || !playerId) return;
    const nextMode = nowPlaying.repeat === 0 ? 2 : nowPlaying.repeat === 2 ? 1 : 0;
    await LyrionAPI.setRepeat(playerId, nextMode as 0 | 1 | 2);
    useAppStore.setState((state) => ({
      nowPlaying: { ...state.nowPlaying, repeat: nextMode },
    }));
  };

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <Group gap="sm">
      {/* Playback controls */}
      <Group gap="xs">
        <ActionIcon
          size="md"
          variant="subtle"
          onClick={handlePrevious}
          disabled={!currentPlayer}
        >
          <IconPlayerSkipBack size={18} />
        </ActionIcon>

        <ActionIcon
          size="lg"
          variant="filled"
          onClick={handlePlay}
          disabled={!currentPlayer}
        >
          {nowPlaying.playing ? (
            <IconPlayerPause size={20} />
          ) : (
            <IconPlayerPlay size={20} />
          )}
        </ActionIcon>

        <ActionIcon
          size="md"
          variant="subtle"
          onClick={handleStop}
          disabled={!currentPlayer}
        >
          <IconPlayerStop size={18} />
        </ActionIcon>

        <ActionIcon
          size="md"
          variant="subtle"
          onClick={handleNext}
          disabled={!currentPlayer}
        >
          <IconPlayerSkipForward size={18} />
        </ActionIcon>
      </Group>

      {/* Shuffle and Repeat */}
      <Group gap="xs">
        <ActionIcon
          size="md"
          variant="subtle"
          onClick={handleShuffle}
          disabled={!currentPlayer}
          color={nowPlaying.shuffle > 0 ? 'blue' : 'gray'}
        >
          <IconArrowsShuffle size={16} />
        </ActionIcon>

        <ActionIcon
          size="md"
          variant="subtle"
          onClick={handleRepeat}
          disabled={!currentPlayer}
          color={nowPlaying.repeat > 0 ? 'blue' : 'gray'}
        >
          {nowPlaying.repeat === 1 ? (
            <IconRepeatOnce size={16} />
          ) : (
            <IconRepeat size={16} />
          )}
        </ActionIcon>
      </Group>

      {/* Track info */}
      {nowPlaying.track && (
        <Text size="sm" style={{ maxWidth: 200 }} truncate>
          {nowPlaying.track.title || 'Unknown'} - {nowPlaying.track.artist || 'Unknown'}
        </Text>
      )}

      {/* Time */}
      {nowPlaying.duration > 0 && (
        <Text size="xs" c="dimmed">
          {formatTime(nowPlaying.position)} / {formatTime(nowPlaying.duration)}
        </Text>
      )}

      {/* Volume */}
      <Group gap="xs" style={{ width: 120 }}>
        <IconVolume size={16} />
        <Slider
          value={nowPlaying.volume}
          onChange={handleVolumeChange}
          min={0}
          max={100}
          size="xs"
          style={{ flex: 1 }}
          disabled={!currentPlayer}
        />
        <Text size="xs" c="dimmed" style={{ width: 30 }}>
          {nowPlaying.volume}%
        </Text>
      </Group>
    </Group>
  );
}
