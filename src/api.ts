import { invoke } from "@tauri-apps/api/core"; // Tauri v1: '@tauri-apps/api/tauri'
import type { BlockableApp, Session } from "./types";

const inTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Used only when running `vite` in a plain browser, so you can work on the UI.
const MOCK_APPS: BlockableApp[] = [
  { id: "chrome.exe", name: "Google Chrome" },
  { id: "discord.exe", name: "Discord" },
  { id: "steam.exe", name: "Steam" },
  { id: "spotify.exe", name: "Spotify" },
  { id: "slack.exe", name: "Slack" },
  { id: "telegram.exe", name: "Telegram" },
  { id: "whatsapp.exe", name: "WhatsApp" },
  { id: "netflix.exe", name: "Netflix" },
  { id: "epicgameslauncher.exe", name: "Epic Games Launcher" },
];

export async function getApps(): Promise<BlockableApp[]> {
  if (!inTauri) return MOCK_APPS;
  return invoke<BlockableApp[]>("get_apps");
}

export async function loadSessions(): Promise<Session[]> {
  if (!inTauri) return JSON.parse(localStorage.getItem("sessions") ?? "[]");
  return invoke<Session[]>("load_sessions");
}

export async function saveSessions(sessions: Session[]): Promise<void> {
  if (!inTauri) {
    localStorage.setItem("sessions", JSON.stringify(sessions));
    return;
  }
  await invoke("save_sessions", { sessions });
}
