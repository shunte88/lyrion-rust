import { ReactNode, useEffect } from 'react';
import { useSpring, animated, config } from 'react-spring';
import { useDrag } from '@use-gesture/react';

interface BottomSheetProps {
  open: boolean;
  onClose: () => void;
  snapPoints: (number | string)[]; // e.g., [60, 300, '80%']
  currentSnapPoint: number; // Index of current snap point
  onSnapPointChange: (index: number) => void;
  children: ReactNode;
  showBackdrop?: boolean;
}

export function BottomSheet({
  open,
  onClose,
  snapPoints,
  currentSnapPoint,
  onSnapPointChange,
  children,
  showBackdrop = true,
}: BottomSheetProps) {
  const windowHeight = typeof window !== 'undefined' ? window.innerHeight : 800;

  // Convert snap points to pixel values
  const getSnapPointValue = (point: number | string): number => {
    if (typeof point === 'number') return point;
    if (typeof point === 'string' && point.endsWith('%')) {
      const percent = parseFloat(point) / 100;
      return windowHeight * percent;
    }
    return 0;
  };

  const snapPointValues = snapPoints.map(getSnapPointValue);
  const currentHeight = snapPointValues[currentSnapPoint] || 0;

  const [{ y }, api] = useSpring(() => ({
    y: open ? windowHeight - currentHeight : windowHeight,
    config: config.stiff,
  }));

  // Update position when snap point changes
  useEffect(() => {
    if (open) {
      api.start({ y: windowHeight - currentHeight });
    }
  }, [currentSnapPoint, open, currentHeight, windowHeight, api]);

  // Update position when opened/closed
  useEffect(() => {
    api.start({ y: open ? windowHeight - currentHeight : windowHeight });
  }, [open, currentHeight, windowHeight, api]);

  const bind = useDrag(
    ({ last, movement: [, my], velocity: [, vy], direction: [, dy] }) => {
      // Only drag down from top of sheet
      if (my < 0) return;

      if (last) {
        // Determine which snap point to snap to
        const currentY = windowHeight - currentHeight;
        const newY = currentY + my;
        const currentHeightFromBottom = windowHeight - newY;

        // If dragging fast, snap in direction of movement
        if (Math.abs(vy) > 0.5) {
          if (dy > 0 && currentSnapPoint > 0) {
            // Dragging down - go to lower snap point
            onSnapPointChange(currentSnapPoint - 1);
          } else if (dy < 0 && currentSnapPoint < snapPoints.length - 1) {
            // Dragging up - go to higher snap point
            onSnapPointChange(currentSnapPoint + 1);
          } else if (dy > 0 && currentSnapPoint === 0) {
            // Close if dragging down from minimum
            onClose();
          }
        } else {
          // Find nearest snap point
          let nearestIndex = 0;
          let nearestDistance = Infinity;

          snapPointValues.forEach((value, index) => {
            const distance = Math.abs(value - currentHeightFromBottom);
            if (distance < nearestDistance) {
              nearestDistance = distance;
              nearestIndex = index;
            }
          });

          // Close if dragged below minimum snap point
          if (currentHeightFromBottom < snapPointValues[0] / 2) {
            onClose();
          } else {
            onSnapPointChange(nearestIndex);
          }
        }

        api.start({ y: windowHeight - snapPointValues[currentSnapPoint] });
      } else {
        // Follow finger during drag
        api.start({ y: (windowHeight - currentHeight) + my, immediate: true });
      }
    },
    {
      from: () => [0, y.get()],
      filterTaps: true,
      bounds: { top: 0 },
      rubberband: true,
    }
  );

  const backdropSpring = useSpring({
    opacity: open ? 0.5 : 0,
    pointerEvents: open ? 'auto' : 'none' as any,
    config: config.stiff,
  });

  return (
    <>
      {/* Backdrop */}
      {showBackdrop && (
        <animated.div
          onClick={onClose}
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            backgroundColor: 'black',
            zIndex: 999,
            ...backdropSpring,
          }}
        />
      )}

      {/* Bottom Sheet */}
      <animated.div
        {...bind()}
        style={{
          position: 'fixed',
          left: 0,
          right: 0,
          bottom: 0,
          height: windowHeight,
          transform: y.to((val) => `translateY(${val}px)`),
          touchAction: 'none',
          zIndex: 1000,
          backgroundColor: 'var(--mantine-color-dark-7)',
          borderTopLeftRadius: '16px',
          borderTopRightRadius: '16px',
          boxShadow: '0 -4px 20px rgba(0, 0, 0, 0.3)',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
      >
        {/* Drag Handle */}
        <div
          style={{
            padding: '12px 0',
            display: 'flex',
            justifyContent: 'center',
            cursor: 'grab',
            flexShrink: 0,
          }}
        >
          <div
            style={{
              width: '40px',
              height: '4px',
              backgroundColor: 'var(--mantine-color-dark-4)',
              borderRadius: '2px',
            }}
          />
        </div>

        {/* Content */}
        <div style={{ flex: 1, overflow: 'auto' }}>
          {children}
        </div>
      </animated.div>
    </>
  );
}
