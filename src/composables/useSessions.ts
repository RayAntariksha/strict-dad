import { ref, watch } from "vue";
import type { Session } from "../types";
import * as api from "../api";

export const SNAP = 15; // minutes
export const MIN_LEN = 15; // minutes
export const DAY = 1440;

// Module-level state: every component that calls useSessions() shares it.
const sessions = ref<Session[]>([]);
let loaded = false;
let timer: number | undefined;

// Debounced persistence to the Rust backend (drafts are never saved).
watch(
  sessions,
  () => {
    if (!loaded) return;
    clearTimeout(timer);
    timer = window.setTimeout(() => {
      const clean = sessions.value
        .filter((s) => !s.draft)
        .map(({ draft, ...rest }) => rest);
      api.saveSessions(clean).catch(console.error);
    }, 400);
  },
  { deep: true },
);

export function useSessions() {
  async function load() {
    sessions.value = await api.loadSessions();
    loaded = true;
  }

  function canPlace(
    date: string,
    start: number,
    end: number,
    ignoreId?: string,
  ) {
    if (start < 0 || end > DAY || end - start < MIN_LEN) return false;
    return !sessions.value.some(
      (s) =>
        s.id !== ignoreId && s.date === date && s.start < end && start < s.end,
    );
  }

  /** Create a draft session (default 1h) at `start`, shrunk to fit before the next session. */
  function add(date: string, start: number): Session | null {
    start = Math.min(start, DAY - MIN_LEN);
    const sameDay = sessions.value.filter((s) => s.date === date);
    if (sameDay.some((s) => s.start <= start && start < s.end)) return null;
    const nextStart = Math.min(
      DAY,
      ...sameDay.filter((s) => s.start > start).map((s) => s.start),
    );
    const end = Math.min(start + 60, nextStart);
    if (end - start < MIN_LEN) return null;
    const s: Session = {
      id: crypto.randomUUID(),
      date,
      start,
      end,
      apps: [],
      draft: true,
    };
    sessions.value.push(s);
    return s;
  }

  function update(id: string, patch: Partial<Session>) {
    const s = sessions.value.find((x) => x.id === id);
    if (s) Object.assign(s, patch);
  }

  function remove(id: string) {
    sessions.value = sessions.value.filter((s) => s.id !== id);
  }

  return { sessions, load, canPlace, add, update, remove };
}
