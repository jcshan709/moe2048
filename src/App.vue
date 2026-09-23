<script setup lang="ts">
/**
 * App shell: header, controls, board.
 *
 * All game state comes from `useGame`, which in turn comes from the Rust engine;
 * this component only lays things out and forwards input.
 */

import { onMounted } from "vue";

import GameBoard from "./components/GameBoard.vue";
import ScorePanel from "./components/ScorePanel.vue";
import { useGame } from "./composables/useGame";
import { useKeyboard } from "./composables/useKeyboard";
import { useSwipe } from "./composables/useSwipe";

const { tiles, score, moves, best, status, canUndo, error, shaking, start, play, undo, newGame, keepPlaying } =
  useGame();

const swipe = useSwipe(play);

useKeyboard({ onDirection: play, onUndo: undo, onRestart: newGame });

onMounted(() => {
  void start();
});
</script>

<template>
  <div class="app">
    <header class="app__header">
      <div class="app__brand">
        <h1 class="app__title">Moe2048</h1>
        <p class="app__subtitle">合并方块，拼出 2048</p>
      </div>

      <div class="app__scores">
        <ScorePanel label="得分" :value="score" />
        <ScorePanel label="最高分" :value="best" />
      </div>
    </header>

    <div class="app__toolbar">
      <p class="app__meta">已走 {{ moves }} 步</p>
      <div class="app__actions">
        <button type="button" class="button" :disabled="!canUndo" @click="undo">撤销</button>
        <button type="button" class="button button--primary" @click="newGame">新游戏</button>
      </div>
    </div>

    <p v-if="error" class="app__error" role="alert">
      与游戏引擎通信失败：{{ error }}
    </p>

    <GameBoard
      :tiles="tiles"
      :status="status"
      :can-undo="canUndo"
      :shaking="shaking"
      @touchstart="swipe.onTouchStart"
      @touchend="swipe.onTouchEnd"
      @keep-playing="keepPlaying"
      @restart="newGame"
      @undo="undo"
    />

    <footer class="app__footer">
      <span><kbd>↑</kbd><kbd>↓</kbd><kbd>←</kbd><kbd>→</kbd> 或 <kbd>W</kbd><kbd>A</kbd><kbd>S</kbd><kbd>D</kbd> 移动</span>
      <span><kbd>Ctrl</kbd>+<kbd>Z</kbd> 撤销 · <kbd>R</kbd> 重新开始 · 触屏可滑动</span>
    </footer>
  </div>
</template>
