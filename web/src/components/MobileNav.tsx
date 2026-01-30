import { Group, UnstyledButton, Stack, Text } from '@mantine/core';
import {
  IconPlayerPlay,
  IconSettings,
  IconDisc,
  IconMicrophone,
  IconList,
} from '@tabler/icons-react';
import { useNavigate, useLocation } from 'react-router-dom';
import { Haptics } from '../utils/haptics';

export function MobileNav() {
  const navigate = useNavigate();
  const location = useLocation();

  const navItems = [
    { path: '/now-playing', label: 'Playing', icon: IconPlayerPlay },
    { path: '/tracks', label: 'Tracks', icon: IconList },
    { path: '/albums', label: 'Albums', icon: IconDisc },
    { path: '/artists', label: 'Artists', icon: IconMicrophone },
    { path: '/settings', label: 'Settings', icon: IconSettings },
  ];

  return (
    <Group
      justify="space-around"
      gap={0}
      style={{
        position: 'fixed',
        bottom: 0,
        left: 0,
        right: 0,
        height: 60,
        backgroundColor: 'var(--mantine-color-dark-7)',
        borderTop: '1px solid var(--mantine-color-dark-5)',
        boxShadow: '0 -2px 8px rgba(0, 0, 0, 0.15)',
        zIndex: 100,
      }}
    >
      {navItems.map((item) => {
        const Icon = item.icon;
        const isActive = location.pathname === item.path;

        return (
          <UnstyledButton
            key={item.path}
            onClick={() => {
              Haptics.selection();
              navigate(item.path);
            }}
            style={{
              flex: 1,
              height: '100%',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <Stack gap={2} align="center">
              <Icon
                size={24}
                stroke={1.5}
                color={isActive ? 'var(--mantine-color-blue-5)' : 'var(--mantine-color-gray-5)'}
              />
              <Text
                size="10px"
                fw={isActive ? 600 : 400}
                c={isActive ? 'blue' : 'dimmed'}
              >
                {item.label}
              </Text>
            </Stack>
          </UnstyledButton>
        );
      })}
    </Group>
  );
}
