export interface BlockableApp {
  id: string; // stable identifier your Rust side uses (process name, bundle id, ...)
  name: string; // display name
  icon?: string; // optional data URL (e.g. "data:image/png;base64,...")
}

export interface Session {
  id: string;
  date: string; // local date, "YYYY-MM-DD"
  start: number; // minutes from midnight (0-1440)
  end: number; // minutes from midnight (0-1440)
  apps: string[]; // BlockableApp ids
  draft?: boolean; // UI-only: session created but not yet confirmed in the dialog
}
