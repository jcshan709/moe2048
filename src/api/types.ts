/**
 * TypeScript mirror of the Rust wire format.
 *
 * These names are pinned by `the_wire_format_matches_what_the_frontend_expects`
 * in `src-tauri/src/game/state.rs`, so a rename on either side fails a test
 * instead of silently producing `undefined` at runtime.
 */

/** The four slides. Matches `Direction` in `src-tauri/src/game/direction.rs`. */
export type Direction = "up" | "down" | "left" | "right";

/** Matches `GameStatus`. */
export type GameStatus = "playing" | "won" | "lost";

/** A board coordinate. */
export interface Pos {
  row: number;
  col: number;
}

/** A tile as the engine sees it, with a stable identity used as the render key. */
export interface TileView {
  id: number;
  value: number;
  at: Pos;
}

/** A tile that survived a move and changed cell. */
export interface SlideEvent {
  id: number;
  from: Pos;
  to: Pos;
}

/** Two tiles collapsing into one. */
export interface MergeEvent {
  /** Cell the merged tile appears in. */
  at: Pos;
  /** Identity of the merged tile. */
  id: number;
  value: number;
  /**
   * The two consumed tiles, each already pointing at `at` so they can be slid
   * into place before being removed.
   */
  consumed: [SlideEvent, SlideEvent];
}

/** A new tile appearing in an empty cell. */
export interface SpawnEvent {
  id: number;
  value: number;
  at: Pos;
}

/** A complete snapshot of the visible game. */
export interface GameState {
  /** Cell values, `0` for empty. */
  board: number[][];
  /** Every tile on the board, with identities. */
  tiles: TileView[];
  score: number;
  moves: number;
  status: GameStatus;
  canUndo: boolean;
}

/** The result of one slide. */
export interface MoveOutcome {
  state: GameState;
  /** `false` when the direction was blocked; then nothing moved or spawned. */
  moved: boolean;
  gained: number;
  slides: SlideEvent[];
  merges: MergeEvent[];
  spawn: SpawnEvent | null;
}
