/**
 * The best score, persisted in the webview's own storage.
 *
 * A Tauri window keeps its origin's storage in the app's data directory, so this
 * survives restarts without pulling in a storage plugin. Every access is guarded
 * because storage throws when it is disabled or full, and losing a high score
 * must never take the game down with it.
 */

import { ref, type Ref } from "vue";

const STORAGE_KEY = "moe2048.best-score";

function read(): number {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw === null) {
      return 0;
    }
    const parsed = Number.parseInt(raw, 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : 0;
  } catch {
    return 0;
  }
}

export interface BestScore {
  best: Ref<number>;
  /** Raises the stored best score when `score` beats it. */
  record: (score: number) => void;
}

export function useBestScore(): BestScore {
  const best = ref(read());

  function record(score: number): void {
    if (score <= best.value) {
      return;
    }
    best.value = score;
    try {
      window.localStorage.setItem(STORAGE_KEY, String(score));
    } catch {
      // The score is still correct for this session.
    }
  }

  return { best, record };
}
