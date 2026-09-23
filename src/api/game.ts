/**
 * Typed wrappers around the Rust commands.
 *
 * Nothing else in the frontend calls `invoke` directly, so the command names and
 * payload shapes live in exactly one place.
 */

import { invoke } from "@tauri-apps/api/core";

import type { Direction, GameState, MoveOutcome } from "./types";

/**
 * Normalizes a rejection from `invoke` into an `Error`.
 *
 * Rust returns the serialized `ApiError`, but a transport failure rejects with
 * something else entirely, so this has to cope with both.
 */
function toError(cause: unknown): Error {
  if (cause instanceof Error) {
    return cause;
  }
  if (typeof cause === "string") {
    return new Error(cause);
  }
  if (cause && typeof cause === "object" && "message" in cause) {
    const { message } = cause as { message: unknown };
    if (typeof message === "string") {
      return new Error(message);
    }
  }
  return new Error(`The game backend failed: ${String(cause)}`);
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    throw toError(cause);
  }
}

/** Starts a fresh game. */
export function newGame(): Promise<GameState> {
  return call<GameState>("new_game");
}

/** Reads the current position, used to resync after a reload. */
export function fetchState(): Promise<GameState> {
  return call<GameState>("game_state");
}

/**
 * Slides the board.
 *
 * A blocked direction is not an error: it comes back with `moved: false`.
 */
export function makeMove(direction: Direction): Promise<MoveOutcome> {
  return call<MoveOutcome>("make_move", { direction });
}

/** Takes back the last move, or resolves `null` when there is nothing to undo. */
export function undoMove(): Promise<GameState | null> {
  return call<GameState | null>("undo_move");
}

/** Dismisses the win overlay so play can continue. */
export function keepPlaying(): Promise<GameState> {
  return call<GameState>("keep_playing");
}
