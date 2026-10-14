<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { BlockableApp, Session } from "../types";
import { DAY, MIN_LEN, useSessions } from "../composables/useSessions";

const props = defineProps<{
  session: Session;
  apps: BlockableApp[];
  loading?: boolean;
  appsError?: string;
}>();
const emit = defineEmits<{
  (e: "save", patch: { start: number; end: number; apps: string[] }): void;
  (e: "cancel"): void;
  (e: "delete"): void;
}>();

const { canPlace } = useSessions();
const isNew = computed(() => !!props.session.draft);

/* <input type="time"> can't express 24:00, so "23:59" stands for end of day. */
const pad = (n: number) => String(n).padStart(2, "0");
const toHM = (m: number) =>
  m >= DAY ? "23:59" : `${pad(Math.floor(m / 60))}:${pad(m % 60)}`;
const fromHM = (v: string) => {
  const [h, m] = v.split(":").map(Number);
  const total = h * 60 + m;
  return total >= DAY - 1 ? DAY : total;
};

const start = ref(toHM(props.session.start));
const end = ref(toHM(props.session.end));
const selected = ref(new Set(props.session.apps));
const query = ref("");
const search = ref<HTMLInputElement>();
onMounted(() => search.value?.focus());

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return q
    ? props.apps.filter((a) => a.name.toLowerCase().includes(q))
    : props.apps;
});

function toggle(id: string) {
  const next = new Set(selected.value);
  next.has(id) ? next.delete(id) : next.add(id);
  selected.value = next;
}
function selectShown() {
  selected.value = new Set([
    ...selected.value,
    ...filtered.value.map((a) => a.id),
  ]);
}
const clearAll = () => (selected.value = new Set());

const error = computed(() => {
  const s = fromHM(start.value);
  const e = fromHM(end.value);
  if (e - s < MIN_LEN)
    return "End time must be at least 15 minutes after the start.";
  if (!canPlace(props.session.date, s, e, props.session.id))
    return "This overlaps another session on the same day.";
  return "";
});
const canSave = computed(() => !error.value && selected.value.size > 0);

const dateLabel = computed(() => {
  const [y, m, d] = props.session.date.split("-").map(Number);
  return new Date(y, m - 1, d).toLocaleDateString([], {
    weekday: "long",
    month: "long",
    day: "numeric",
  });
});

function save() {
  if (!canSave.value) return;
  emit("save", {
    start: fromHM(start.value),
    end: fromHM(end.value),
    apps: [...selected.value],
  });
}
</script>

<template>
  <div
    class="scrim"
    tabindex="-1"
    @keydown.esc="emit('cancel')"
    @mousedown.self="emit('cancel')"
  >
    <form
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="dlg-title"
      @submit.prevent="save"
    >
      <header>
        <h2 id="dlg-title">
          {{ isNew ? "New block session" : "Edit block session" }}
        </h2>
        <p class="date">{{ dateLabel }}</p>
        <div class="times">
          <input
            v-model="start"
            type="time"
            step="900"
            aria-label="Start time"
            required
          />
          <span>to</span>
          <input
            v-model="end"
            type="time"
            step="900"
            aria-label="End time"
            required
          />
        </div>
        <p v-if="error" class="error" role="alert">{{ error }}</p>
      </header>

      <section class="picker">
        <div class="tools">
          <input
            ref="search"
            v-model="query"
            type="search"
            placeholder="Search apps"
            aria-label="Search apps"
          />
          <button type="button" class="link" @click="selectShown">
            Select shown
          </button>
          <button type="button" class="link" @click="clearAll">Clear</button>
        </div>

        <p v-if="loading" class="empty">Loading apps…</p>
        <p v-else-if="appsError" class="empty error" role="alert">
          {{ appsError }}
        </p>
        <p v-else-if="!apps.length" class="empty">
          No apps found. Check that the backend's get_apps command returns a
          list.
        </p>
        <p v-else-if="!filtered.length" class="empty">
          No apps match “{{ query }}”.
        </p>

        <ul v-else class="list">
          <li v-for="a in filtered" :key="a.id">
            <label :class="{ on: selected.has(a.id) }">
              <input
                type="checkbox"
                :checked="selected.has(a.id)"
                @change="toggle(a.id)"
              />
              <span class="icon">
                <img v-if="a.icon" :src="a.icon" alt="" />
                <template v-else>{{ a.name.charAt(0) }}</template>
              </span>
              <span class="name">{{ a.name }}</span>
            </label>
          </li>
        </ul>
      </section>

      <footer>
        <button
          v-if="!isNew"
          type="button"
          class="danger"
          @click="emit('delete')"
        >
          Delete
        </button>
        <span class="count">{{ selected.size }} selected</span>
        <button type="button" class="ghost" @click="emit('cancel')">
          Cancel
        </button>
        <button type="submit" class="primary" :disabled="!canSave">
          Save session
        </button>
      </footer>
    </form>
  </div>
</template>

<style scoped>
.scrim {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  background: rgba(23, 33, 43, 0.4);
  font-family: "Segoe UI Variable Text", "SF Pro Text", system-ui, sans-serif;
}
.dialog {
  display: flex;
  flex-direction: column;
  width: min(440px, calc(100vw - 32px));
  max-height: min(640px, calc(100vh - 32px));
  border-radius: 14px;
  background: #fff;
  color: #17212b;
  box-shadow: 0 24px 60px rgba(23, 33, 43, 0.35);
  animation: pop 0.14s ease-out;
}
@keyframes pop {
  from {
    transform: translateY(6px) scale(0.98);
    opacity: 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .dialog {
    animation: none;
  }
}

header {
  padding: 20px 22px 12px;
}
h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}
.date {
  margin: 2px 0 14px;
  font-size: 13px;
  color: #6b7785;
}
.times {
  display: flex;
  align-items: center;
  gap: 10px;
  color: #6b7785;
  font-size: 13px;
}
.times input,
.tools input {
  padding: 8px 10px;
  border: 1px solid #d5dce4;
  border-radius: 8px;
  font: inherit;
  font-size: 14px;
  color: #17212b;
  background: #fff;
}
.times input {
  font-variant-numeric: tabular-nums;
}
.times input:focus,
.tools input:focus {
  outline: 2px solid #234e70;
  outline-offset: 0;
  border-color: transparent;
}
.error {
  margin: 10px 0 0;
  font-size: 13px;
  color: #b3262b;
}

.picker {
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: 0 22px;
  flex: 1;
}
.tools {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}
.tools input {
  flex: 1;
  min-width: 0;
}
.link {
  border: 0;
  background: none;
  padding: 4px;
  font: inherit;
  font-size: 13px;
  color: #234e70;
  cursor: pointer;
}
.link:hover {
  text-decoration: underline;
}

.list {
  margin: 0;
  padding: 0;
  list-style: none;
  overflow-y: auto;
  min-height: 120px;
  border-top: 1px solid #e1e6ec;
}
.list label {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 9px 6px;
  border-bottom: 1px solid #eef1f5;
  cursor: pointer;
}
.list label:hover {
  background: #f5f7fa;
}
.list label.on {
  background: #eef4fa;
}
.list input {
  width: 16px;
  height: 16px;
  accent-color: #234e70;
}
.icon {
  display: grid;
  place-items: center;
  flex: none;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  overflow: hidden;
  background: #e1e6ec;
  font-size: 13px;
  font-weight: 600;
  color: #234e70;
}
.icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.name {
  font-size: 14px;
}
.empty {
  margin: 20px 0;
  font-size: 13px;
  color: #6b7785;
}

footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 22px 18px;
}
.count {
  margin-right: auto;
  font-size: 13px;
  color: #6b7785;
}
footer .danger + .count {
  margin-left: 4px;
}
button {
  font: inherit;
  font-size: 14px;
  cursor: pointer;
}
.primary {
  padding: 8px 16px;
  border: 0;
  border-radius: 8px;
  background: #234e70;
  color: #fff;
  font-weight: 600;
}
.primary:disabled {
  background: #b8c4d0;
  cursor: not-allowed;
}
.ghost {
  padding: 8px 14px;
  border: 0;
  border-radius: 8px;
  background: none;
  color: #17212b;
}
.ghost:hover {
  background: #f2f4f7;
}
.danger {
  padding: 8px 12px;
  border: 0;
  border-radius: 8px;
  background: none;
  color: #b3262b;
}
.danger:hover {
  background: #fdeeee;
}
button:focus-visible {
  outline: 2px solid #234e70;
  outline-offset: 2px;
}
</style>
