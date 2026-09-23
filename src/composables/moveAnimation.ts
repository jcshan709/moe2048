/**
 * Turning an engine move into a tile list the browser can animate.
 *
 * These are pure functions with no Vue or Tauri dependency, so the trickiest part
 * of the frontend — the slide/merge/spawn choreography — can be exercised directly
 * (see `scripts/check-move-animation.ts`) instead of only through the UI.
 *
 * A move is rendered in two steps so a merge reads correctly:
 *
 * 1. {@link beginMove} points every surviving tile at its new cell and sends each
 *    merged-away tile to the cell it merges into, marked `dying`.
 * 2. Once the transition has run, {@link settleMove} drops the `dying` tiles and
 *    adds the results — the merged tiles and the new spawn — which pop in.
 */

import type { GameState, MoveOutcome } from "../api/types";

/** One-shot presentation hint, applied when a tile first appears. */
export type TilePhase = "idle" | "new" | "merged" | "dying";

/** A tile as rendered, at an integer cell coordinate. */
export interface DisplayTile {
  id: number;
  value: number;
  row: number;
  col: number;
  phase: TilePhase;
}

/** Builds the tile list from a full engine snapshot. */
export function tilesFromState(state: GameState, phase: TilePhase): DisplayTile[] {
  return state.tiles.map((tile) => ({
    id: tile.id,
    value: tile.value,
    row: tile.at.row,
    col: tile.at.col,
    phase,
  }));
}

/**
 * Step 1: applies the movement half of a move.
 *
 * Returns a new list; the input is not modified. Tiles are matched by identity, so
 * a tile the UI never rendered is ignored rather than corrupting the list.
 *
 * Only `row`/`col`/`phase` change, which is what lets CSS animate the difference.
 */
export function beginMove(tiles: readonly DisplayTile[], outcome: MoveOutcome): DisplayTile[] {
  const moved = new Map<number, { row: number; col: number; phase: TilePhase }>();

  for (const slide of outcome.slides) {
    moved.set(slide.id, { row: slide.to.row, col: slide.to.col, phase: "idle" });
  }
  for (const merge of outcome.merges) {
    for (const consumed of merge.consumed) {
      // Both tiles travel to the merge cell before disappearing.
      moved.set(consumed.id, { row: consumed.to.row, col: consumed.to.col, phase: "dying" });
    }
  }

  return tiles.map((tile) => {
    const next = moved.get(tile.id);
    return next === undefined ? tile : { ...tile, ...next };
  });
}

/**
 * Step 2: retires the merged-away tiles and pops in the results.
 *
 * Returns a new list containing exactly the tiles the engine reports, so any drift
 * between the UI and the engine is corrected here rather than compounding.
 */
export function settleMove(tiles: readonly DisplayTile[], outcome: MoveOutcome): DisplayTile[] {
  const consumed = new Set<number>();
  for (const merge of outcome.merges) {
    for (const tile of merge.consumed) {
      consumed.add(tile.id);
    }
  }

  const next = tiles.filter((tile) => !consumed.has(tile.id));

  for (const merge of outcome.merges) {
    next.push({
      id: merge.id,
      value: merge.value,
      row: merge.at.row,
      col: merge.at.col,
      phase: "merged",
    });
  }

  if (outcome.spawn !== null) {
    const { id, value, at } = outcome.spawn;
    next.push({ id, value, row: at.row, col: at.col, phase: "new" });
  }

  return next;
}
