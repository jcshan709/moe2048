/**
 * The game view-model.
 *
 * The Rust engine owns every rule; this composable only turns its output into
 * something renderable:
 *
 * 1. a move is sent to the engine and returns a full description of what changed;
 * 2. surviving tiles get their new cell, which the CSS transition animates;
 * 3. merged-away tiles are slid onto their destination and faded out;
 * 4. once the slide has settled, those tiles are dropped and the results
 *    (merged tiles and the new spawn) pop in.
 *
 * Input is locked while a slide is in flight, and at most one direction is
 * buffered, so mashing the arrow keys stays smooth instead of tearing the
 * animation.
 */

import { ref, type Ref } from "vue";

import { fetchState, keepPlaying as resume, makeMove, newGame as startGame, undoMove } from "../api/game";
import type { Direction, GameState, GameStatus, MoveOutcome } from "../api/types";
import { beginMove, settleMove, tilesFromState, type DisplayTile, type TilePhase } from "./moveAnimation";
import { useBestScore } from "./useBestScore";

/** How long the slide transition runs. Must match `--move-ms` in the stylesheet. */
const MOVE_MS = 120;

/** How long the "you cannot move that way" wobble lasts. */
const SHAKE_MS = 320;

export type { DisplayTile, TilePhase };

export interface Game {
  tiles: Ref<DisplayTile[]>;
  score: Ref<number>;
  moves: Ref<number>;
  best: Ref<number>;
  status: Ref<GameStatus>;
  canUndo: Ref<boolean>;
  error: Ref<string | null>;
  shaking: Ref<boolean>;
  /** Loads the initial position. Call once, on mount. */
  start: () => Promise<void>;
  play: (direction: Direction) => void;
  undo: () => Promise<void>;
  newGame: () => Promise<void>;
  keepPlaying: () => Promise<void>;
}

export function useGame(): Game {
  const tiles = ref<DisplayTile[]>([]);
  const score = ref(0);
  const moves = ref(0);
  const status = ref<GameStatus>("playing");
  const canUndo = ref(false);
  const error = ref<string | null>(null);
  const shaking = ref(false);
  const { best, record } = useBestScore();

  /** True while a slide is animating; further input is buffered instead. */
  let busy = false;
  /**
   * False until the first snapshot arrives.
   *
   * Input before then would be applied to an empty tile list, so the board would
   * only partially render until the next move. The engine has a live game from
   * the start, so this is purely a guard against a fast first keypress.
   */
  let ready = false;
  /** At most one pending direction, so held or mashed keys never queue up. */
  let queued: Direction | null = null;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  let shakeTimer: ReturnType<typeof setTimeout> | undefined;

  /** Replaces the whole tile list from an authoritative engine snapshot. */
  function resync(state: GameState, phase: TilePhase = "idle"): void {
    tiles.value = tilesFromState(state, phase);
    score.value = state.score;
    moves.value = state.moves;
    status.value = state.status;
    canUndo.value = state.canUndo;
    record(state.score);
    // Every full snapshot leaves the UI consistent, so input is safe from here on.
    ready = true;
  }

  /** Releases the input lock and runs whatever the player pressed meanwhile. */
  function release(): void {
    busy = false;
    const next = queued;
    queued = null;
    if (next !== null) {
      play(next);
    }
  }

  function fail(cause: unknown): void {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }

  function wobble(): void {
    shaking.value = true;
    clearTimeout(shakeTimer);
    shakeTimer = setTimeout(() => {
      shaking.value = false;
    }, SHAKE_MS);
  }

  /**
   * Drops the merged-away tiles and pops in the results.
   *
   * Runs after the slide so the merge reads as two tiles meeting before a new one
   * appears, rather than the result appearing a step early.
   */
  function settle(outcome: MoveOutcome): void {
    tiles.value = settleMove(tiles.value, outcome);

    score.value = outcome.state.score;
    moves.value = outcome.state.moves;
    status.value = outcome.state.status;
    canUndo.value = outcome.state.canUndo;
    record(outcome.state.score);

    release();
  }

  function play(direction: Direction): void {
    // The overlay owns the board until the player dismisses it, and a move before
    // the first snapshot would render only part of the board.
    if (!ready || status.value !== "playing") {
      return;
    }
    if (busy) {
      queued = direction;
      return;
    }
    busy = true;

    void makeMove(direction)
      .then((outcome) => {
        if (!outcome.moved) {
          wobble();
          release();
          return;
        }

        // Reposition for the slide; CSS animates the change. `settle` runs after
        // the transition so the merge reads as two tiles meeting first.
        tiles.value = beginMove(tiles.value, outcome);

        clearTimeout(settleTimer);
        settleTimer = setTimeout(() => settle(outcome), MOVE_MS);
      })
      .catch((cause: unknown) => {
        fail(cause);
        release();
      });
  }

  async function start(): Promise<void> {
    try {
      resync(await fetchState(), "new");
    } catch (cause) {
      fail(cause);
    }
  }

  async function undo(): Promise<void> {
    if (busy) {
      return;
    }
    busy = true;
    queued = null;
    try {
      const state = await undoMove();
      if (state) {
        resync(state);
      }
    } catch (cause) {
      fail(cause);
    }
    release();
  }

  async function newGame(): Promise<void> {
    queued = null;
    busy = true;
    clearTimeout(settleTimer);
    try {
      resync(await startGame(), "new");
    } catch (cause) {
      fail(cause);
    }
    release();
  }

  async function keepPlaying(): Promise<void> {
    try {
      resync(await resume());
    } catch (cause) {
      fail(cause);
    }
  }

  return {
    tiles,
    score,
    moves,
    best,
    status,
    canUndo,
    error,
    shaking,
    start,
    play,
    undo,
    newGame,
    keepPlaying,
  };
}
