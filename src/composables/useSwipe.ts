/**
 * Touch controls: a swipe across the board slides it.
 *
 * Returns handlers to bind in the template rather than attaching its own
 * listeners, so there is no element ref to wait for and no lifecycle to unwind.
 * The board also needs `touch-action: none` so the gesture is not stolen by
 * scrolling.
 */

import type { Direction } from "../api/types";

/** Shorter drags are treated as taps, not swipes. */
const MIN_DISTANCE_PX = 24;

export interface SwipeHandlers {
  onTouchStart: (event: TouchEvent) => void;
  onTouchEnd: (event: TouchEvent) => void;
}

export function useSwipe(onDirection: (direction: Direction) => void): SwipeHandlers {
  let origin: { x: number; y: number } | null = null;

  function onTouchStart(event: TouchEvent): void {
    const touch = event.changedTouches[0];
    origin = touch ? { x: touch.clientX, y: touch.clientY } : null;
  }

  function onTouchEnd(event: TouchEvent): void {
    const start = origin;
    origin = null;

    const touch = event.changedTouches[0];
    if (start === null || !touch) {
      return;
    }

    const dx = touch.clientX - start.x;
    const dy = touch.clientY - start.y;
    if (Math.max(Math.abs(dx), Math.abs(dy)) < MIN_DISTANCE_PX) {
      return;
    }

    // The dominant axis wins, so a slightly diagonal swipe still reads clearly.
    if (Math.abs(dx) > Math.abs(dy)) {
      onDirection(dx > 0 ? "right" : "left");
    } else {
      onDirection(dy > 0 ? "down" : "up");
    }
  }

  return { onTouchStart, onTouchEnd };
}
