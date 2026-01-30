import { NavLink, Stack, Text } from '@mantine/core';
import {
  IconPlayerPlay,
  IconSettings,
  IconDisc,
  IconMicrophone,
  IconList,
  IconMoodSmile
} from '@tabler/icons-react';
import { useNavigate, useLocation } from 'react-router-dom';

export function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();

  const navItems = [
    { path: '/now-playing', label: 'Now Playing', icon: IconPlayerPlay },
    { path: '/tracks', label: 'Tracks', icon: IconList },
    { path: '/albums', label: 'Albums', icon: IconDisc },
    { path: '/artists', label: 'Artists', icon: IconMicrophone },
    { path: '/genres', label: 'Genres', icon: IconMoodSmile },
    { path: '/settings', label: 'Settings', icon: IconSettings },
  ];

  return (
    <Stack p="md" gap="xs">
      <Text size="xs" fw={700} c="dimmed" mb="sm">
        NAVIGATION
      </Text>

      {navItems.map((item) => (
        <NavLink
          key={item.path}
          label={item.label}
          leftSection={<item.icon size={20} />}
          active={location.pathname === item.path}
          onClick={() => navigate(item.path)}
        />
      ))}
    </Stack>
  );
}
