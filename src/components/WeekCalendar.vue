<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { BlockableApp, Session } from "../types";
import { DAY, MIN_LEN, SNAP, useSessions } from "../composables/useSessions";

const props = defineProps<{ apps: BlockableApp[] }>();
const emit = defineEmits<{ (e: "open", id: string): void }>();

const HOUR_H = 56; // px per hour
const { sessions, add, update, canPlace } = useSessions();

/* ---------- dates ---------- */
const pad = (n: number) => String(n).padStart(2, "0");
const toKey = (d: Date) =>
  `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
function startOfWeek(d: Date) {
  const x = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  x.setDate(x.getDate() - ((x.getDay() + 6) % 7)); // Monday
  return x;
}

const weekStart = ref(startOfWeek(new Date()));
const now = ref(new Date());
let clock: number;
onMounted(
  () => (clock = window.setInterval(() => (now.value = new Date()), 30_000)),
);
onBeforeUnmount(() => clearInterval(clock));

const days = computed(() =>
  Array.from({ length: 7 }, (_, i) => {
    const d = new Date(weekStart.value);
    d.setDate(d.getDate() + i);
    return {
      key: toKey(d),
      dow: d.toLocaleDateString([], { weekday: "short" }),
      dom: d.getDate(),
      isToday: toKey(d) === toKey(now.value),
    };
  }),
);

const rangeLabel = computed(() => {
  const start = weekStart.value;
  const end = new Date(start);
  end.setDate(end.getDate() + 6);
  const m = (d: Date) => d.toLocaleDateString([], { month: "long" });
  return start.getMonth() === end.getMonth()
    ? `${m(start)} ${end.getFullYear()}`
    : `${m(start).slice(0, 3)} – ${m(end).slice(0, 3)} ${end.getFullYear()}`;
});

function shift(weeks: number) {
  const d = new Date(weekStart.value);
  d.setDate(d.getDate() + weeks * 7);
  weekStart.value = d;
}
const goToday = () => (weekStart.value = startOfWeek(new Date()));

/* ---------- formatting ---------- */
const hourLabel = (h: number) =>
  new Date(2000, 0, 1, h).toLocaleTimeString([], { hour: "numeric" });
const fmt = (m: number) =>
  new Date(2000, 0, 1, 0, m % DAY).toLocaleTimeString([], {
    hour: "numeric",
    minute: "2-digit",
  });
const appById = computed(() => new Map(props.apps.map((a) => [a.id, a])));

function titleOf(s: Session) {
  const names = s.apps.map((id) => appById.value.get(id)?.name ?? id);
  if (!names.length) return "Choose apps…";
  return names.length <= 2
    ? names.join(", ")
    : `${names.slice(0, 2).join(", ")} +${names.length - 2}`;
}

/* ---------- layout ---------- */
const placed = computed(() =>
  sessions.value
    .map((s) => ({ s, col: days.value.findIndex((d) => d.key === s.date) }))
    .filter((x) => x.col >= 0),
);

function boxStyle(s: Session, col: number) {
  return {
    top: `${(s.start / 60) * HOUR_H}px`,
    height: `${((s.end - s.start) / 60) * HOUR_H - 2}px`,
    left: `calc(${col} * 100% / 7 + 3px)`,
    width: "calc(100% / 7 - 8px)",
  };
}
const nowTop = computed(
  () =>
    `${((now.value.getHours() * 60 + now.value.getMinutes()) / 60) * HOUR_H}px`,
);

const scroller = ref<HTMLElement>();
onMounted(() => {
  const h = Math.max(0, Math.min(now.value.getHours() - 2, 7));
  if (scroller.value) scroller.value.scrollTop = h * HOUR_H;
});

/* ---------- click blank space to add ---------- */
function onColumnClick(e: MouseEvent, key: string) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const minutes =
    Math.floor(((e.clientY - rect.top) / HOUR_H) * (60 / SNAP)) * SNAP;
  const created = add(key, minutes);
  if (created) emit("open", created.id);
}

/* ---------- drag to move / resize ---------- */
type Mode = "move" | "top" | "bottom";
interface DragState {
  id: string;
  mode: Mode;
  x0: number;
  y0: number;
  start0: number;
  end0: number;
  moved: boolean;
}
let drag: DragState | null = null;
const dragId = ref<string | null>(null);
const gridEl = ref<HTMLElement>();
const clamp = (v: number, lo: number, hi: number) =>
  Math.min(hi, Math.max(lo, v));

function onDown(e: PointerEvent, s: Session, mode: Mode) {
  if (e.button !== 0) return;
  e.stopPropagation();
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  drag = {
    id: s.id,
    mode,
    x0: e.clientX,
    y0: e.clientY,
    start0: s.start,
    end0: s.end,
    moved: false,
  };
}

function onMove(e: PointerEvent) {
  if (!drag || !gridEl.value) return;
  const s = sessions.value.find((x) => x.id === drag!.id);
  if (!s) return;
  const dx = e.clientX - drag.x0;
  const dy = e.clientY - drag.y0;
  if (!drag.moved && Math.hypot(dx, dy) < 4) return; // still a click
  drag.moved = true;
  dragId.value = s.id;

  const delta = Math.round((dy / HOUR_H) * (60 / SNAP)) * SNAP;

  if (drag.mode === "move") {
    const len = drag.end0 - drag.start0;
    const start = clamp(drag.start0 + delta, 0, DAY - len);
    const rect = gridEl.value.getBoundingClientRect();
    const col = clamp(
      Math.floor(((e.clientX - rect.left) / rect.width) * 7),
      0,
      6,
    );
    const date = days.value[col].key;
    if (canPlace(date, start, start + len, s.id))
      update(s.id, { date, start, end: start + len });
  } else if (drag.mode === "top") {
    const start = clamp(drag.start0 + delta, 0, drag.end0 - MIN_LEN);
    if (canPlace(s.date, start, s.end, s.id)) update(s.id, { start });
  } else {
    const end = clamp(drag.end0 + delta, drag.start0 + MIN_LEN, DAY);
    if (canPlace(s.date, s.start, end, s.id)) update(s.id, { end });
  }
}

function onUp() {
  if (drag && !drag.moved && drag.mode === "move") emit("open", drag.id); // plain click = edit
  drag = null;
  dragId.value = null;
}
</script>

<template>
  <section
    class="cal"
    :class="{ dragging: dragId }"
    :style="{ '--hh': HOUR_H + 'px' }"
  >
    <header class="bar">
      <button class="btn" @click="goToday">Today</button>
      <button class="icon-btn" aria-label="Previous week" @click="shift(-1)">
        ‹
      </button>
      <button class="icon-btn" aria-label="Next week" @click="shift(1)">
        ›
      </button>
      <h1>{{ rangeLabel }}</h1>
    </header>

    <div ref="scroller" class="scroller">
      <div class="head">
        <div class="gutter" />
        <div
          v-for="d in days"
          :key="d.key"
          class="head-day"
          :class="{ today: d.isToday }"
        >
          <span class="dow">{{ d.dow }}</span>
          <span class="dom">{{ d.dom }}</span>
        </div>
      </div>

      <div class="body">
        <div class="gutter">
          <span
            v-for="h in 23"
            :key="h"
            class="hour"
            :style="{ top: h * HOUR_H + 'px' }"
          >
            {{ hourLabel(h) }}
          </span>
        </div>

        <div ref="gridEl" class="grid">
          <div
            v-for="d in days"
            :key="d.key"
            class="col"
            :class="{ today: d.isToday }"
            @click="onColumnClick($event, d.key)"
          >
            <div v-if="d.isToday" class="now" :style="{ top: nowTop }" />
          </div>

          <!-- One shared layer, so a session keeps its DOM node while dragging between days. -->
          <div class="layer">
            <article
              v-for="{ s, col } in placed"
              :key="s.id"
              class="session"
              :class="{
                short: s.end - s.start < 45,
                draft: s.draft,
                active: dragId === s.id,
              }"
              :style="boxStyle(s, col)"
              tabindex="0"
              @pointerdown="onDown($event, s, 'move')"
              @pointermove="onMove"
              @pointerup="onUp"
              @pointercancel="onUp"
              @keydown.enter="emit('open', s.id)"
            >
              <div class="handle top" @pointerdown="onDown($event, s, 'top')" />
              <p class="title">{{ titleOf(s) }}</p>
              <p class="time">{{ fmt(s.start) }} – {{ fmt(s.end) }}</p>
              <div v-if="s.end - s.start >= 75 && s.apps.length" class="chips">
                <span v-for="id in s.apps.slice(0, 5)" :key="id" class="chip">
                  <img
                    v-if="appById.get(id)?.icon"
                    :src="appById.get(id)!.icon"
                    alt=""
                  />
                  <template v-else>{{
                    (appById.get(id)?.name ?? id).charAt(0)
                  }}</template>
                </span>
              </div>
              <div
                class="handle bottom"
                @pointerdown="onDown($event, s, 'bottom')"
              />
            </article>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.cal {
  --paper: #f2f4f7;
  --surface: #ffffff;
  --ink: #17212b;
  --muted: #6b7785;
  --line: #e1e6ec;
  --block: #234e70;
  --lock: #f2a93b;
  --now: #e5484d;
  --gutter: 64px;

  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--surface);
  color: var(--ink);
  font-family: "Segoe UI Variable Text", "SF Pro Text", system-ui, sans-serif;
}
.cal.dragging {
  user-select: none;
  cursor: grabbing;
}

.bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--line);
}
.bar h1 {
  margin: 0 0 0 12px;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
}
.btn {
  padding: 7px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
  font-size: 14px;
  cursor: pointer;
}
.icon-btn {
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--ink);
  font-size: 22px;
  line-height: 1;
  cursor: pointer;
}
.btn:hover,
.icon-btn:hover {
  background: var(--paper);
}
.btn:focus-visible,
.icon-btn:focus-visible {
  outline: 2px solid var(--block);
  outline-offset: 2px;
}

.scroller {
  flex: 1;
  overflow-y: auto;
  position: relative;
}

.head {
  position: sticky;
  top: 0;
  z-index: 5;
  display: grid;
  grid-template-columns: var(--gutter) repeat(7, 1fr);
  background: var(--surface);
  border-bottom: 1px solid var(--line);
}
.head-day {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 0 10px;
  border-left: 1px solid var(--line);
  color: var(--muted);
  font-size: 12px;
}
.head-day .dom {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  font-size: 20px;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}
.head-day.today .dow {
  color: var(--block);
  font-weight: 600;
}
.head-day.today .dom {
  background: var(--block);
  color: #fff;
}

.body {
  display: grid;
  grid-template-columns: var(--gutter) 1fr;
}
.gutter {
  position: relative;
}
.hour {
  position: absolute;
  right: 8px;
  transform: translateY(-50%);
  font-size: 11px;
  color: var(--muted);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.grid {
  position: relative;
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  height: calc(var(--hh) * 24);
}
.col {
  position: relative;
  border-left: 1px solid var(--line);
  background-image: repeating-linear-gradient(
    to bottom,
    transparent 0,
    transparent calc(var(--hh) - 1px),
    var(--line) calc(var(--hh) - 1px),
    var(--line) var(--hh)
  );
  cursor: cell;
}
.col:hover {
  background-color: #f8f9fb;
}
.col.today {
  background-color: #f4f8fc;
}

.now {
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--now);
  z-index: 2;
  pointer-events: none;
}
.now::before {
  content: "";
  position: absolute;
  left: -6px;
  top: -5px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--now);
}

.layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.session {
  position: absolute;
  box-sizing: border-box;
  padding: 5px 8px 5px 10px;
  overflow: hidden;
  border-radius: 6px;
  border-left: 4px solid var(--lock);
  background: var(--block);
  color: #fff;
  pointer-events: auto;
  cursor: grab;
  touch-action: none;
  box-shadow: 0 1px 2px rgba(23, 33, 43, 0.25);
}
.session:focus-visible {
  outline: 2px solid var(--ink);
  outline-offset: 1px;
}
.session.active {
  z-index: 4;
  box-shadow: 0 6px 18px rgba(23, 33, 43, 0.35);
}
.session.draft {
  background: repeating-linear-gradient(
    135deg,
    #2e5f87 0 8px,
    #285779 8px 16px
  );
}
.session p {
  margin: 0;
  font-size: 12px;
  line-height: 1.35;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.session .title {
  font-weight: 600;
  font-size: 13px;
}
.session .time {
  opacity: 0.85;
  font-variant-numeric: tabular-nums;
}
.session.short {
  display: flex;
  gap: 8px;
  align-items: baseline;
  padding-top: 2px;
  padding-bottom: 2px;
}

.chips {
  display: flex;
  gap: 4px;
  margin-top: 6px;
}
.chip {
  display: grid;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 5px;
  overflow: hidden;
  background: rgba(255, 255, 255, 0.18);
  font-size: 11px;
  font-weight: 600;
}
.chip img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.handle {
  position: absolute;
  left: 0;
  right: 0;
  height: 8px;
  cursor: ns-resize;
}
.handle.top {
  top: 0;
}
.handle.bottom {
  bottom: 0;
}
.handle.bottom::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 2px;
  width: 22px;
  height: 3px;
  margin-left: -11px;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.5);
  opacity: 0;
  transition: opacity 0.12s;
}
.session:hover .handle.bottom::after {
  opacity: 1;
}

@media (max-width: 720px) {
  .cal {
    --gutter: 44px;
  }
}
@media (prefers-reduced-motion: reduce) {
  * {
    transition: none !important;
  }
}
</style>
