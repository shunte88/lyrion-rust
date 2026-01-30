import { createTheme, MantineColorsTuple } from '@mantine/core';

const lyrionTeal: MantineColorsTuple = [
  '#e3faf3', '#c8f0e3', '#a8e6d1', '#89dcbf', '#6dd2ad',
  '#589e81', // primary (light mode)
  '#4a8a6f', '#3d765d', '#30624b', '#234e39',
];

const lyrionLime: MantineColorsTuple = [
  '#fcffe8', '#f9ffcc', '#f4ffaa', '#effe88', '#ebfe66',
  '#e3fdc4', // primary (dark mode)
  '#d4e8b0', '#c5d39c', '#b6be88', '#a7a974',
];

export const lyrionTheme = createTheme({
  primaryColor: 'lyrionTeal',
  colors: {
    lyrionTeal,
    lyrionLime,
  },
  black: '#081f22', // navy background for dark mode
});
