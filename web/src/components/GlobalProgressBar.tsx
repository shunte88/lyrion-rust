import { Slider, Box } from '@mantine/core';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';

export function GlobalProgressBar() {
  const { nowPlaying, currentPlayer } = useAppStore();
  const playerId = currentPlayer?.mac || currentPlayer?.id || currentPlayer?.uuid || '';

  const progress = nowPlaying.duration > 0
    ? (nowPlaying.position / nowPlaying.duration) * 100
    : 0;

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

  // Show progress bar if track is loaded and playing
  if (!nowPlaying.track || !nowPlaying.playing) {
    return null;
  }

  return (
    <Box px="md" py={4} style={{ backgroundColor: 'var(--mantine-color-dark-7)' }}>
      <Slider
        value={progress}
        onChange={handleSeek}
        min={0}
        max={100}
        size="xs"
        disabled={!currentPlayer || !nowPlaying.duration}
        label={(val) => formatTime((val / 100) * (nowPlaying.duration || 0))}
        styles={{
          root: { width: '100%' },
        }}
      />
    </Box>
  );
}
