use minutes_core::{Config, ContentType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct AppState {
    pub recording: Arc<AtomicBool>,
}

/// Start recording in a background thread.
pub fn start_recording(_app_handle: tauri::AppHandle, recording: Arc<AtomicBool>) {
    recording.store(true, Ordering::Relaxed);

    let config = Config::load();
    let wav_path = minutes_core::pid::current_wav_path();

    if let Err(e) = minutes_core::pid::create() {
        eprintln!("Failed to create PID: {}", e);
        recording.store(false, Ordering::Relaxed);
        return;
    }

    eprintln!("Recording started...");

    let stop_flag = recording.clone();
    match minutes_core::capture::record_to_wav(&wav_path, stop_flag, &config) {
        Ok(()) => match minutes_core::process(&wav_path, ContentType::Meeting, None, &config) {
            Ok(result) => {
                eprintln!(
                    "Saved: {} ({} words)",
                    result.path.display(),
                    result.word_count
                );
            }
            Err(e) => eprintln!("Pipeline error: {}", e),
        },
        Err(e) => eprintln!("Capture error: {}", e),
    }

    minutes_core::pid::remove().ok();
    if wav_path.exists() {
        std::fs::remove_file(&wav_path).ok();
    }
    recording.store(false, Ordering::Relaxed);
}

#[tauri::command]
pub fn cmd_status(state: tauri::State<AppState>) -> serde_json::Value {
    let recording = state.recording.load(Ordering::Relaxed);
    let status = minutes_core::pid::status();
    serde_json::json!({
        "recording": recording || status.recording,
        "pid": status.pid,
    })
}

#[tauri::command]
pub fn cmd_list_meetings(limit: Option<usize>) -> serde_json::Value {
    let config = Config::load();
    let filters = minutes_core::search::SearchFilters {
        content_type: None,
        since: None,
        attendee: None,
    };
    // Empty query returns all files
    match minutes_core::search::search("", &config, &filters) {
        Ok(results) => {
            let limited: Vec<_> = results.into_iter().take(limit.unwrap_or(10)).collect();
            serde_json::to_value(&limited).unwrap_or(serde_json::json!([]))
        }
        Err(_) => serde_json::json!([]),
    }
}

#[tauri::command]
pub fn cmd_search(query: String) -> serde_json::Value {
    let config = Config::load();
    let filters = minutes_core::search::SearchFilters {
        content_type: None,
        since: None,
        attendee: None,
    };
    match minutes_core::search::search(&query, &config, &filters) {
        Ok(results) => serde_json::to_value(&results).unwrap_or(serde_json::json!([])),
        Err(_) => serde_json::json!([]),
    }
}
