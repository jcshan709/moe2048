<script setup lang="ts">
/**
 * A labelled number box, used for the score and the best score.
 */

defineProps<{
  label: string;
  value: number;
}>();
</script>

<template>
  <div class="score">
    <span class="score__label">{{ label }}</span>
    <span class="score__value">{{ value }}</span>
  </div>
</template>
