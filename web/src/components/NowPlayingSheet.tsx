import { useState } from 'react';
import { Stack, Group, Text, Image, ActionIcon, Slider, Paper, Progress } from '@mantine/core';
import {
  IconPlayerPlay,
  IconPlayerPause,
  IconPlayerSkipBack,
  IconPlayerSkipForward,
  IconVolume,
  IconChevronUp,
} from '@tabler/icons-react';
import { useAppStore } from '../services/store';
import { LyrionAPI } from '../services/api';
import { BottomSheet } from './BottomSheet';
import { Haptics } from '../utils/haptics';

interface NowPlayingSheetProps {
  open: boolean;
  onClose: () => void;
}

export function NowPlayingSheet({ open, onClose }: NowPlayingSheetProps) {
  const { nowPlaying, currentPlayer } = useAppStore();
  const [snapPoint, setSnapPoint] = useState(0); // 0 = minimized, 1 = half, 2 = full

  const coverUrl = nowPlaying.track?.has_cover && nowPlaying.track.id
    ? LyrionAPI.getCoverArtUrl(nowPlaying.track.id)
    : undefined;

  const handlePlay = async () => {
    if (!currentPlayer) return;
    Haptics.tap();
    if (nowPlaying.playing) {
      await LyrionAPI.pause(currentPlayer.id);
    } else {
      await LyrionAPI.play(currentPlayer.id);
    }
  };

  const handlePrevious = async () => {
    if (!currentPlayer) return;
    Haptics.tap();
    await LyrionAPI.previous(currentPlayer.id);
  };

  const handleNext = async () => {
    if (!currentPlayer) return;
    Haptics.tap();
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

  // Snap points: minimized (80px), half (400px), full (80%)
  const snapPoints = [80, 400, '80%'];

  return (
    <BottomSheet
      open={open}
      onClose={onClose}
      snapPoints={snapPoints}
      currentSnapPoint={snapPoint}
      onSnapPointChange={setSnapPoint}
      showBackdrop={snapPoint > 0}
    >
      {/* Minimized View (always visible) */}
      {snapPoint === 0 && (
        <div style={{ padding: '0 16px 16px 16px' }}>
          <Group gap="md" wrap="nowrap">
            <Image
              src={coverUrl}
              fallbackSrc="https://placehold.co/48x48/1a1b1e/white?text=♪"
              alt={nowPlaying.track?.title || 'Track'}
              w={48}
              h={48}
              fit="cover"
              radius="sm"
            />

            <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
              <Text size="sm" fw={500} lineClamp={1}>
                {nowPlaying.track?.title || 'No track playing'}
              </Text>
              <Text size="xs" c="dimmed" lineClamp={1}>
                {nowPlaying.track?.artist || 'Unknown Artist'}
              </Text>
            </Stack>

            <ActionIcon
              size="xl"
              variant="filled"
              onClick={handlePlay}
              disabled={!currentPlayer}
              style={{ minWidth: 48, minHeight: 48 }}
            >
              {nowPlaying.playing ? (
                <IconPlayerPause size={24} />
              ) : (
                <IconPlayerPlay size={24} />
              )}
            </ActionIcon>

            <ActionIcon
              size="lg"
              variant="subtle"
              onClick={() => setSnapPoint(2)}
              style={{ minWidth: 44, minHeight: 44 }}
            >
              <IconChevronUp size={24} />
            </ActionIcon>
          </Group>
        </div>
      )}

      {/* Half View (queue preview) */}
      {snapPoint === 1 && (
        <Stack p="md" gap="lg">
          <Group gap="md" wrap="nowrap">
            <Image
              src={coverUrl}
              fallbackSrc="https://placehold.co/80x80/1a1b1e/white?text=♪"
              alt={nowPlaying.track?.title || 'Track'}
              w={80}
              h={80}
              fit="cover"
              radius="md"
            />

            <Stack gap={4} style={{ flex: 1, minWidth: 0 }}>
              <Text size="lg" fw={600} lineClamp={2}>
                {nowPlaying.track?.title || 'No track playing'}
              </Text>
              <Text size="sm" c="dimmed" lineClamp={1}>
                {nowPlaying.track?.artist || 'Unknown Artist'}
              </Text>
              <Text size="sm" c="dimmed" lineClamp={1}>
                {nowPlaying.track?.album || 'Unknown Album'}
              </Text>
            </Stack>
          </Group>

          <Stack gap="xs">
            <Progress value={progress} size="sm" />
            <Group justify="space-between">
              <Text size="xs" c="dimmed">
                {formatTime(nowPlaying.position)}
              </Text>
              <Text size="xs" c="dimmed">
                {formatTime(nowPlaying.duration)}
              </Text>
            </Group>
          </Stack>

          <Group justify="center" gap="xl">
            <ActionIcon
              size={60}
              variant="subtle"
              onClick={handlePrevious}
              disabled={!currentPlayer}
              style={{ minWidth: 60, minHeight: 60 }}
            >
              <IconPlayerSkipBack size={32} />
            </ActionIcon>

            <ActionIcon
              size={80}
              variant="filled"
              onClick={handlePlay}
              disabled={!currentPlayer}
              style={{ minWidth: 80, minHeight: 80 }}
              radius="xl"
            >
              {nowPlaying.playing ? (
                <IconPlayerPause size={40} />
              ) : (
                <IconPlayerPlay size={40} />
              )}
            </ActionIcon>

            <ActionIcon
              size={60}
              variant="subtle"
              onClick={handleNext}
              disabled={!currentPlayer}
              style={{ minWidth: 60, minHeight: 60 }}
            >
              <IconPlayerSkipForward size={32} />
            </ActionIcon>
          </Group>

          {/* Queue Preview */}
          {nowPlaying.playlist.length > 0 && (
            <Paper p="md" radius="md" style={{ backgroundColor: 'var(--mantine-color-dark-6)' }}>
              <Text size="sm" fw={600} mb="xs">
                Up Next ({nowPlaying.playlist.length} tracks)
              </Text>
              <Stack gap="xs">
                {nowPlaying.playlist.slice(nowPlaying.playlist_index + 1, nowPlaying.playlist_index + 4).map((track) => (
                  <Group key={track.id} gap="sm">
                    <Text size="xs" lineClamp={1} style={{ flex: 1 }}>
                      {track.title}
                    </Text>
                    <Text size="xs" c="dimmed">
                      {track.artist}
                    </Text>
                  </Group>
                ))}
              </Stack>
            </Paper>
          )}
        </Stack>
      )}

      {/* Full View (all controls) */}
      {snapPoint === 2 && (
        <Stack p="xl" gap="xl">
          {/* Large Album Art */}
          <Group justify="center">
            <Image
              src={coverUrl}
              fallbackSrc="https://placehold.co/320x320/1a1b1e/white?text=No+Artwork"
              alt={nowPlaying.track?.title || 'Track'}
              w={320}
              h={320}
              fit="cover"
              radius="lg"
              style={{ boxShadow: '0 8px 32px rgba(0, 0, 0, 0.3)' }}
            />
          </Group>

          {/* Track Info */}
          <Stack gap={4} align="center">
            <Text size="xl" fw={700} ta="center" lineClamp={2}>
              {nowPlaying.track?.title || 'No track playing'}
            </Text>
            <Text size="lg" c="dimmed" ta="center" lineClamp={1}>
              {nowPlaying.track?.artist || 'Unknown Artist'}
            </Text>
            <Text size="md" c="dimmed" ta="center" lineClamp={1}>
              {nowPlaying.track?.album || 'Unknown Album'}
            </Text>
          </Stack>

          {/* Progress Bar */}
          <Stack gap="xs">
            <Progress value={progress} size="md" radius="xl" />
            <Group justify="space-between">
              <Text size="sm" c="dimmed">
                {formatTime(nowPlaying.position)}
              </Text>
              <Text size="sm" c="dimmed">
                {formatTime(nowPlaying.duration)}
              </Text>
            </Group>
          </Stack>

          {/* Playback Controls */}
          <Group justify="center" gap="xl">
            <ActionIcon
              size={70}
              variant="subtle"
              onClick={handlePrevious}
              disabled={!currentPlayer}
              style={{ minWidth: 70, minHeight: 70 }}
            >
              <IconPlayerSkipBack size={36} />
            </ActionIcon>

            <ActionIcon
              size={90}
              variant="filled"
              onClick={handlePlay}
              disabled={!currentPlayer}
              style={{ minWidth: 90, minHeight: 90 }}
              radius="xl"
            >
              {nowPlaying.playing ? (
                <IconPlayerPause size={48} />
              ) : (
                <IconPlayerPlay size={48} />
              )}
            </ActionIcon>

            <ActionIcon
              size={70}
              variant="subtle"
              onClick={handleNext}
              disabled={!currentPlayer}
              style={{ minWidth: 70, minHeight: 70 }}
            >
              <IconPlayerSkipForward size={36} />
            </ActionIcon>
          </Group>

          {/* Volume Control */}
          <Group gap="md" style={{ padding: '0 20px' }}>
            <IconVolume size={24} />
            <Slider
              value={nowPlaying.volume}
              onChange={handleVolumeChange}
              min={0}
              max={100}
              style={{ flex: 1 }}
              disabled={!currentPlayer}
              size="lg"
              thumbSize={24}
            />
            <Text size="sm" c="dimmed" style={{ width: 40, textAlign: 'right' }}>
              {nowPlaying.volume}%
            </Text>
          </Group>
        </Stack>
      )}
    </BottomSheet>
  );
}
