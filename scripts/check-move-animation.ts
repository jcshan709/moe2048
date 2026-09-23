/**
 * Checks the tile choreography in `src/composables/moveAnimation.ts`.
 *
 * The frontend has no test runner (adding one would mean another dependency), and
 * Node can run TypeScript directly, so this script is the check:
 *
 *     node scripts/check-move-animation.ts
 *
 * Exit code 0 means every assertion held.
 */

import {
  beginMove,
  settleMove,
  tilesFromState,
  type DisplayTile,
} from "../src/composables/moveAnimation.ts";
import type { MoveOutcome, TileView } from "../src/api/types.ts";

let checks = 0;

function ok(condition: boolean, what: string): void {
  checks += 1;
  if (!condition) {
    throw new Error(`FAILED: ${what}`);
  }
}

function equal<T>(actual: T, expected: T, what: string): void {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  checks += 1;
  if (a !== e) {
    throw new Error(`FAILED: ${what}\n  actual:   ${a}\n  expected: ${e}`);
  }
}

function tile(id: number, value: number, row: number, col: number, phase: DisplayTile["phase"] = "idle"): DisplayTile {
  return { id, value, row, col, phase };
}

function view(id: number, value: number, row: number, col: number): TileView {
  return { id, value, at: { row, col } };
}

/** A slide with no merges: two tiles travel left. */
const slideOnly: MoveOutcome = {
  state: {
    board: [
      [2, 4, 0, 0],
      [0, 0, 0, 0],
      [0, 0, 0, 0],
      [0, 0, 0, 0],
    ],
    tiles: [view(1, 2, 0, 0), view(2, 4, 0, 1), view(9, 2, 0, 3)],
    score: 0,
    moves: 1,
    status: "playing",
    canUndo: true,
  },
  moved: true,
  gained: 0,
  slides: [
    { id: 1, from: { row: 0, col: 1 }, to: { row: 0, col: 0 } },
    { id: 2, from: { row: 0, col: 3 }, to: { row: 0, col: 1 } },
  ],
  merges: [],
  spawn: { id: 9, value: 2, at: { row: 0, col: 3 } },
};

/** [4, 4] collapsing into an 8 at column 0. */
const merge: MoveOutcome = {
  state: {
    board: [
      [8, 0, 0, 2],
      [0, 0, 0, 0],
      [0, 0, 0, 0],
      [0, 0, 0, 0],
    ],
    tiles: [view(30, 8, 0, 0), view(20, 2, 0, 3)],
    score: 8,
    moves: 2,
    status: "playing",
    canUndo: true,
  },
  moved: true,
  gained: 8,
  slides: [],
  merges: [
    {
      at: { row: 0, col: 0 },
      id: 30,
      value: 8,
      consumed: [
        { id: 10, from: { row: 0, col: 0 }, to: { row: 0, col: 0 } },
        { id: 11, from: { row: 0, col: 1 }, to: { row: 0, col: 0 } },
      ],
    },
  ],
  spawn: { id: 20, value: 2, at: { row: 0, col: 3 } },
};

// --- step 1: sliding -------------------------------------------------------

{
  const start = [tile(1, 2, 0, 1), tile(2, 4, 0, 3)];
  const after = beginMove(start, slideOnly);

  equal(after.map((t) => [t.id, t.row, t.col]), [[1, 0, 0], [2, 0, 1]], "surviving tiles reach their destination");
  equal(after.map((t) => t.phase), ["idle", "idle"], "sliding tiles are not marked as new or merged");
  equal(start.map((t) => t.col), [1, 3], "the input list is left untouched");
  equal(after.length, 2, "no tile is added or removed before settling");
}

// --- step 1: merging -------------------------------------------------------

{
  const start = [tile(10, 4, 0, 0), tile(11, 4, 0, 1)];
  const after = beginMove(start, merge);

  equal(
    after.map((t) => [t.id, t.row, t.col, t.phase]),
    [[10, 0, 0, "dying"], [11, 0, 0, "dying"]],
    "both consumed tiles travel to the merge cell and are marked dying",
  );
  const spawned = after.find((t) => t.id === 30);
  ok(spawned === undefined, "the merged tile is not present until the slide settles");
}

// --- step 2: settling -----------------------------------------------------

{
  const sliding = beginMove([tile(1, 2, 0, 1), tile(2, 4, 0, 3)], slideOnly);
  const settled = settleMove(sliding, slideOnly);

  equal(
    settled.map((t) => [t.id, t.value, t.row, t.col, t.phase]),
    [[1, 2, 0, 0, "idle"], [2, 4, 0, 1, "idle"], [9, 2, 0, 3, "new"]],
    "settling adds the spawn and keeps the survivors",
  );
}

{
  const merging = beginMove([tile(10, 4, 0, 0), tile(11, 4, 0, 1)], merge);
  const settled = settleMove(merging, merge);

  equal(
    settled.map((t) => [t.id, t.value, t.row, t.col, t.phase]),
    [[30, 8, 0, 0, "merged"], [20, 2, 0, 3, "new"]],
    "settling retires the consumed tiles and pops in the merge plus the spawn",
  );
  equal(
    settled.filter((t) => t.phase === "dying").length,
    0,
    "no dying tile survives settling",
  );
}

// --- identity handling -----------------------------------------------------

{
  // A tile the UI never rendered must be ignored, not corrupt the list.
  const settled = settleMove([], slideOnly);
  equal(settled.length, 1, "an empty list settles down to just the spawn");
  equal(settled[0].id, 9, "the spawn survives");
}

{
  // Every tile in the settled list must be unique, or Vue would warn about
  // duplicate keys and render one of them twice.
  const merging = beginMove([tile(10, 4, 0, 0), tile(11, 4, 0, 1)], merge);
  const settled = settleMove(merging, merge);
  const ids = settled.map((t) => t.id);
  equal(new Set(ids).size, ids.length, "settled tile ids are unique");
}

// --- full move over a real-looking sequence --------------------------------

{
  // Slide, merge, then settle; the tile count must obey the rule the engine
  // guarantees: survivors - merges + spawn.
  const start = [tile(10, 4, 0, 0), tile(11, 4, 0, 1), tile(12, 2, 1, 2)];
  const sliding = beginMove(start, merge);
  const settled = settleMove(sliding, merge);

  const expected = start.length - merge.merges.length + (merge.spawn === null ? 0 : 1);
  equal(settled.length, expected, "tile count matches survivors minus merges plus spawn");
  ok(
    settled.every((t) => t.row >= 0 && t.row < 4 && t.col >= 0 && t.col < 4),
    "every settled tile stays inside the board",
  );
}

// --- building from a snapshot ----------------------------------------------

{
  const built = tilesFromState(slideOnly.state, "new");
  equal(
    built.map((t) => [t.id, t.value, t.row, t.col, t.phase]),
    [[1, 2, 0, 0, "new"], [2, 4, 0, 1, "new"], [9, 2, 0, 3, "new"]],
    "a snapshot builds the tile list with the requested phase",
  );
  equal(tilesFromState(slideOnly.state, "idle").map((t) => t.phase), ["idle", "idle", "idle"], "phase is honoured");
}

console.log(`move animation: ${checks} checks passed`);
