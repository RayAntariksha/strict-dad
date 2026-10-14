use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use sysinfo::{ProcessesToUpdate, System};

const POLL_INTERVAL: Duration = Duration::from_millis(750);

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    id: String,
    date: String,
    start: u16,
    end: u16,
    apps: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BlockableApp {
    id: String,
    name: String,
}

fn data_file() -> Result<PathBuf, String> {
    let data_dir = match std::env::var_os("XDG_DATA_HOME") {
        Some(path) => PathBuf::from(path),
        None => {
            let home = std::env::var_os("HOME")
                .ok_or_else(|| "Could not determine the user's home directory".to_string())?;
            PathBuf::from(home).join(".local/share")
        }
    };
    Ok(data_dir.join("strict-dad").join("sessions.json"))
}

pub fn load_sessions() -> Result<Vec<Session>, String> {
    let path = data_file()?;
    match fs::read(&path) {
        Ok(contents) => serde_json::from_slice(&contents)
            .map_err(|error| format!("Could not parse {}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("Could not read {}: {error}", path.display())),
    }
}

pub fn save_sessions(sessions: Vec<Session>) -> Result<(), String> {
    let path = data_file()?;
    let parent = path
        .parent()
        .ok_or_else(|| "Session storage path has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;

    let temporary_path = path.with_extension(format!("{}.tmp", std::process::id()));
    let contents = serde_json::to_vec_pretty(&sessions)
        .map_err(|error| format!("Could not serialize sessions: {error}"))?;
    fs::write(&temporary_path, contents)
        .map_err(|error| format!("Could not write {}: {error}", temporary_path.display()))?;
    if let Err(error) = fs::rename(&temporary_path, &path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "Could not replace session file {}: {error}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn get_apps() -> Result<Vec<BlockableApp>, String> {
    let mut roots = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        roots.push(PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/applications"));
    }
    let data_dirs =
        std::env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    roots.extend(std::env::split_paths(&data_dirs).map(|path| path.join("applications")));

    let mut apps_by_id = std::collections::HashMap::new();
    for root in roots {
        if root.is_dir() {
            read_desktop_files(&root, &mut apps_by_id, 0)?;
        }
    }

    let mut apps: Vec<_> = apps_by_id.into_values().collect();
    apps.sort_by(|left: &BlockableApp, right: &BlockableApp| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(apps)
}

#[cfg(not(target_os = "linux"))]
pub fn get_apps() -> Result<Vec<BlockableApp>, String> {
    Err("Installed app discovery is currently supported on Linux only".to_string())
}

#[cfg(target_os = "linux")]
fn read_desktop_files(
    directory: &Path,
    apps: &mut std::collections::HashMap<String, BlockableApp>,
    depth: usize,
) -> Result<(), String> {
    if depth > 8 {
        return Ok(());
    }
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("Could not read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not read app entry: {error}"))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect {}: {error}", path.display()))?;
        if file_type.is_dir() {
            read_desktop_files(&path, apps, depth + 1)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "desktop")
        {
            if let Some(app) = parse_desktop_file(&path)? {
                apps.entry(app.id.clone()).or_insert(app);
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn parse_desktop_file(path: &Path) -> Result<Option<BlockableApp>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    let mut in_entry = false;
    let mut is_application = false;
    let mut hidden = false;
    let mut name = None;
    let mut exec = None;

    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "Type" => is_application = value == "Application",
                "NoDisplay" | "Hidden" => hidden |= value.eq_ignore_ascii_case("true"),
                "Name" if name.is_none() => name = Some(value.to_string()),
                "Exec" => exec = Some(value.to_string()),
                _ => {}
            }
        }
    }

    if !is_application || hidden {
        return Ok(None);
    }
    let Some(name) = name.filter(|name| !name.trim().is_empty()) else {
        return Ok(None);
    };
    let Some(exec) = exec else {
        return Ok(None);
    };
    let Some(executable) = desktop_exec_program(&exec) else {
        return Ok(None);
    };
    let id = Path::new(&executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&executable)
        .to_lowercase();
    if id.is_empty() {
        return Ok(None);
    }
    Ok(Some(BlockableApp { id, name }))
}

#[cfg(target_os = "linux")]
fn desktop_exec_program(exec: &str) -> Option<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for character in exec.chars() {
        if escaped {
            token.push(character);
            escaped = false;
        } else {
            match character {
                '\\' => escaped = true,
                '"' => in_quotes = !in_quotes,
                '%' => {
                    // Desktop-entry field codes are arguments, not part of the executable.
                    // The executable is always the first token, so ignore codes once a token
                    // has started and let later whitespace end the token.
                }
                character if character.is_whitespace() && !in_quotes => {
                    if !token.is_empty() {
                        tokens.push(std::mem::take(&mut token));
                    }
                }
                _ => token.push(character),
            }
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }

    let mut command_index = 0;
    if tokens.first().is_some_and(|value| value == "env") {
        command_index += 1;
        while tokens
            .get(command_index)
            .is_some_and(|value| value.contains('=') && !value.starts_with('='))
        {
            command_index += 1;
        }
    }
    tokens.get(command_index).cloned()
}

pub fn block_apps(apps: &[String]) -> usize {
    if apps.is_empty() {
        return 0;
    }
    let blocked_names: HashSet<String> = apps.iter().map(|app| app.to_lowercase()).collect();
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut blocked_count = 0;
    for (pid, process) in system.processes() {
        let process_name = process.name().to_string_lossy();
        let executable_name = process
            .exe()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().to_lowercase());
        if blocked_names.contains(&process_name.to_lowercase())
            || executable_name
                .as_ref()
                .is_some_and(|name| blocked_names.contains(name))
        {
            if process.kill() {
                blocked_count += 1;
                eprintln!("Blocked {} (PID {pid})", process.name().to_string_lossy());
                if let Err(error) = notify_rust::Notification::new()
                    .summary("The app was blocked")
                    .body(&format!("{} was blocked", process.name().to_string_lossy()))
                    .timeout(notify_rust::Timeout::Milliseconds(6000))
                    .show()
                {
                    eprintln!("Could not show blocked-app notification: {error}");
                }
            } else {
                eprintln!(
                    "Could not block {} (PID {pid})",
                    process.name().to_string_lossy()
                );
            }
        }
    }
    blocked_count
}

fn active_apps(sessions: &[Session], date: &str, minute: u16) -> Vec<String> {
    sessions
        .iter()
        .filter(|session| session.date == date && session.start <= minute && minute < session.end)
        .flat_map(|session| session.apps.iter().cloned())
        .collect()
}

pub fn start_blocking_monitor() {
    thread::spawn(|| {
        loop {
            match load_sessions() {
                Ok(sessions) => {
                    let now = Local::now();
                    let date = now.format("%Y-%m-%d").to_string();
                    let minute = (now.hour() * 60 + now.minute()) as u16;
                    block_apps(&active_apps(&sessions, &date, minute));
                }
                Err(error) => eprintln!("Could not load sessions for app blocking: {error}"),
            }
            thread::sleep(POLL_INTERVAL);
        }
    });
}

use chrono::Timelike;

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{Session, active_apps, desktop_exec_program, parse_desktop_file};
    use std::fs;

    #[test]
    fn detects_only_apps_in_an_active_session() {
        let sessions = vec![Session {
            id: "session".into(),
            date: "2026-10-14".into(),
            start: 60,
            end: 120,
            apps: vec!["firefox".into()],
        }];
        assert_eq!(active_apps(&sessions, "2026-10-14", 60), ["firefox"]);
        assert!(active_apps(&sessions, "2026-10-14", 120).is_empty());
        assert!(active_apps(&sessions, "2026-10-15", 90).is_empty());
    }

    #[test]
    fn parses_desktop_exec_programs() {
        assert_eq!(
            desktop_exec_program(r#"env GTK_THEME=dark "/usr/bin/my browser" %U"#),
            Some("/usr/bin/my browser".into())
        );
        assert_eq!(desktop_exec_program("firefox %u"), Some("firefox".into()));
    }

    #[test]
    fn excludes_hidden_desktop_entries() {
        let path = std::env::temp_dir().join(format!("strict-dad-{}.desktop", std::process::id()));
        fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=hidden-app\nNoDisplay=true\n",
        )
        .unwrap();
        assert!(parse_desktop_file(&path).unwrap().is_none());
        fs::remove_file(path).unwrap();
    }
}
