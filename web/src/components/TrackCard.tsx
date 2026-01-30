import { Card, Group, Stack, Text, ActionIcon, Image, Badge } from '@mantine/core';
import { IconPlayerPlay, IconDots } from '@tabler/icons-react';
import { LyrionAPI } from '../services/api';
import { Haptics } from '../utils/haptics';
import type { Track } from '../types/api';

interface TrackCardProps {
  track: Track;
  trackNumber: number;
  onPlay: (track: Track) => void;
}

export function TrackCard({ track, onPlay }: TrackCardProps) {
  const formatDuration = (seconds?: number) => {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <Card
      shadow="sm"
      padding="md"
      radius="md"
      withBorder
      style={{
        minHeight: '80px',
        cursor: 'pointer',
      }}
    >
      <Group gap="md" wrap="nowrap">
        {/* Cover Art */}
        <Image
          src={track.has_cover ? LyrionAPI.getCoverArtUrl(track.id) : undefined}
          fallbackSrc="https://placehold.co/60x60/1a1b1e/white?text=♪"
          alt={track.title || 'Track'}
          w={60}
          h={60}
          fit="cover"
          radius="sm"
        />

        {/* Track Info */}
        <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
          <Text size="sm" fw={500} lineClamp={1}>
            {track.title || 'Unknown Title'}
          </Text>
          <Text size="xs" c="dimmed" lineClamp={1}>
            {track.artist || 'Unknown Artist'}
          </Text>
          <Group gap="xs">
            {track.lossless ? (
              <Badge size="xs" variant="light" color="green">
                FLAC
              </Badge>
            ) : (
              <Badge size="xs" variant="light" color="blue">
                {track.content_type?.toUpperCase() || 'MP3'}
              </Badge>
            )}
            <Text size="xs" c="dimmed">
              {formatDuration(track.secs)}
            </Text>
          </Group>
        </Stack>

        {/* Actions */}
        <Stack gap="xs" align="center">
          <ActionIcon
            variant="filled"
            size="lg"
            radius="xl"
            onClick={(e) => {
              e.stopPropagation();
              Haptics.tap();
              onPlay(track);
            }}
            style={{ minWidth: 44, minHeight: 44 }} // Touch-friendly
          >
            <IconPlayerPlay size={20} />
          </ActionIcon>
          <ActionIcon
            variant="subtle"
            size="md"
            onClick={(e) => {
              e.stopPropagation();
              // TODO: Open options menu
            }}
            style={{ minWidth: 44, minHeight: 44 }} // Touch-friendly
          >
            <IconDots size={18} />
          </ActionIcon>
        </Stack>
      </Group>
    </Card>
  );
}
