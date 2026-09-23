/**
 * Keyboard controls.
 *
 * Arrow keys and WASD slide the board, Ctrl+Z takes a move back and R starts
 * over. Bound to the window so the board does not need focus.
 */

import { onMounted, onUnmounted } from "vue";

import type { Direction } from "../api/types";

const BY_KEY: Record<string, Direction | undefined> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  w: "up",
  s: "down",
  a: "left",
  d: "right",
};

export interface KeyboardHandlers {
  onDirection: (direction: Direction) => void;
  onUndo: () => void;
  onRestart: () => void;
}

export function useKeyboard(handlers: KeyboardHandlers): void {
  function handle(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.altKey || event.metaKey) {
      return;
    }

    // Single characters arrive as "w"/"W"; named keys keep their own spelling.
    const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;

    if (event.ctrlKey) {
      if (key === "z") {
        event.preventDefault();
        handlers.onUndo();
      }
      return;
    }

    const direction = BY_KEY[key];
    if (direction !== undefined) {
      // Otherwise the arrow keys scroll the window.
      event.preventDefault();
      handlers.onDirection(direction);
      return;
    }

    if (key === "r") {
      event.preventDefault();
      handlers.onRestart();
    }
  }

  onMounted(() => window.addEventListener("keydown", handle));
  onUnmounted(() => window.removeEventListener("keydown", handle));
}
