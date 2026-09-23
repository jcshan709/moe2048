<script setup lang="ts">
/**
 * One tile.
 *
 * Two nested elements on purpose: the outer one owns the grid position (which
 * transitions when the engine moves the tile), the inner one owns the scale
 * animation for appearing and merging. Keeping them apart stops the two
 * transforms from fighting over the same property.
 */

import { computed, type CSSProperties } from "vue";

import type { DisplayTile } from "../composables/useGame";

const props = defineProps<{ tile: DisplayTile }>();

const label = computed(() => String(props.tile.value));

/** Four-digit values need a smaller face; anything past 2048 gets a neutral one. */
const palette = computed(() => (props.tile.value > 2048 ? "super" : String(props.tile.value)));

const style = computed<CSSProperties>(() => ({
  "--row": props.tile.row,
  "--col": props.tile.col,
}));
</script>

<template>
  <div
    class="tile"
    :class="{ 'tile--dying': tile.phase === 'dying' }"
    :style="style"
    role="img"
    :aria-label="label"
  >
    <div
      class="tile__face"
      :class="[
        `tile__face--${palette}`,
        `tile__face--len-${Math.min(label.length, 5)}`,
        tile.phase === 'new' ? 'tile__face--appearing' : null,
        tile.phase === 'merged' ? 'tile__face--merged' : null,
      ]"
    >
      {{ label }}
    </div>
  </div>
</template>
