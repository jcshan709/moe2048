<script setup lang="ts">
/**
 * The board: a static grid of empty cells plus a layer of absolutely positioned
 * tiles on top.
 *
 * Tiles are positioned from `row`/`col` through CSS custom properties, so a move
 * is just a property change and the browser animates it. That avoids measuring
 * anything in JavaScript and keeps the board responsive at any size.
 *
 * Touch handlers are not declared here: the parent binds them, and Vue's
 * attribute fallthrough puts them on the single root element.
 */

import type { GameStatus } from "../api/types";
import type { DisplayTile } from "../composables/useGame";
import BoardTile from "./BoardTile.vue";
import GameOverlay from "./GameOverlay.vue";

const props = defineProps<{
  tiles: DisplayTile[];
  status: GameStatus;
  canUndo: boolean;
  /** Wobbles the board after a blocked move. */
  shaking: boolean;
}>();

const emit = defineEmits<{
  keepPlaying: [];
  restart: [];
  undo: [];
}>();

/** Empty cells, drawn once as a static backdrop. Mirrors `SIZE` in the engine. */
const BOARD_SIZE = 4;
const CELLS = Array.from({ length: BOARD_SIZE * BOARD_SIZE }, (_, index) => index);
</script>

<template>
  <div class="board-frame">
    <div class="board" :class="{ 'board--shaking': props.shaking }">
      <div class="board__grid" aria-hidden="true">
        <div v-for="cell in CELLS" :key="cell" class="board__cell" />
      </div>

      <BoardTile v-for="tile in props.tiles" :key="tile.id" :tile="tile" />

      <GameOverlay
        v-if="props.status !== 'playing'"
        :status="props.status"
        :can-undo="props.canUndo"
        @keep-playing="emit('keepPlaying')"
        @restart="emit('restart')"
        @undo="emit('undo')"
      />
    </div>
  </div>
</template>
