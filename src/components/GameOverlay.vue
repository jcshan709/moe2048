<script setup lang="ts">
/**
 * Covers the board when the game is won or lost.
 *
 * Kept inside the board so the board keeps its size and nothing below it jumps.
 */

import type { GameStatus } from "../api/types";

const props = defineProps<{
  status: Exclude<GameStatus, "playing">;
  /** Whether taking a move back is still possible. */
  canUndo: boolean;
}>();

const emit = defineEmits<{
  keepPlaying: [];
  restart: [];
  undo: [];
}>();
</script>

<template>
  <div class="overlay" role="alertdialog" aria-live="assertive">
    <template v-if="props.status === 'won'">
      <p class="overlay__title">你赢了！</p>
      <p class="overlay__detail">拼出了 2048</p>
      <div class="overlay__actions">
        <button type="button" class="button button--primary" @click="emit('keepPlaying')">
          继续挑战
        </button>
        <button type="button" class="button" @click="emit('restart')">重新开始</button>
      </div>
    </template>

    <template v-else>
      <p class="overlay__title">无路可走了</p>
      <p class="overlay__detail">棋盘已经填满，且没有可以合并的方块</p>
      <div class="overlay__actions">
        <button
          type="button"
          class="button button--primary"
          :disabled="!props.canUndo"
          @click="emit('undo')"
        >
          撤销一步
        </button>
        <button type="button" class="button" @click="emit('restart')">重新开始</button>
      </div>
    </template>
  </div>
</template>
