<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import WeekCalendar from "./components/WeekCalendar.vue";
import SessionDialog from "./components/SessionDialog.vue";
import { useSessions } from "./composables/useSessions";
import { getApps } from "./api";
import type { BlockableApp } from "./types";

const { sessions, load, update, remove } = useSessions();
const apps = ref<BlockableApp[]>([]);
const appsLoading = ref(true);
const appsError = ref("");
const activeId = ref<string | null>(null);
const active = computed(
  () => sessions.value.find((s) => s.id === activeId.value) ?? null,
);

onMounted(async () => {
  await load();
  try {
    apps.value = await getApps(); // from the Rust backend
  } catch (err) {
    console.error("get_apps failed", err);
    appsError.value = "Could not load installed apps. Please restart the app and try again.";
  } finally {
    appsLoading.value = false;
  }
});

function close() {
  // Closing a brand-new session without saving discards it.
  if (active.value?.draft) remove(active.value.id);
  activeId.value = null;
}
function save(patch: { start: number; end: number; apps: string[] }) {
  if (!active.value) return;
  update(active.value.id, { ...patch, draft: false });
  activeId.value = null;
}
function del() {
  if (active.value) remove(active.value.id);
  activeId.value = null;
}
</script>

<template>
  <main class="app">
    <WeekCalendar :apps="apps" @open="(id) => (activeId = id)" />
    <SessionDialog
      v-if="active"
      :key="active.id"
      :session="active"
      :apps="apps"
      :loading="appsLoading"
      :apps-error="appsError"
      @save="save"
      @cancel="close"
      @delete="del"
    />
  </main>
</template>

<style>
html,
body,
#app {
  height: 100%;
  margin: 0;
}
.app {
  height: 100vh;
}
</style>
