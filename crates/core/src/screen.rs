use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ──────────────────────────────────────────────────────────────
// Screen context capture.
//
// Periodically captures screenshots during a recording session
// to give the LLM visual context about what was on screen.
//
// Privacy model:
//   - Disabled by default (opt-in via config)
//   - Screenshots stored with 0600 permissions
//   - Cleaned up after summarization (configurable)
//   - Never sent anywhere without explicit LLM config
//
// macOS: uses `screencapture -x` (silent, no shutter sound)
// Linux: uses `scrot` or `gnome-screenshot` if available
// ──────────────────────────────────────────────────────────────

/// Start capturing screenshots at a regular interval.
/// Returns a handle that stops capture when dropped.
/// Screenshots are saved as timestamped PNGs in `output_dir`.
pub fn start_capture(
    output_dir: &Path,
    interval: Duration,
    stop_flag: Arc<AtomicBool>,
) -> std::io::Result<ScreenCaptureHandle> {
    std::fs::create_dir_all(output_dir)?;

    // Set directory permissions to 0700 (owner-only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(output_dir, std::fs::Permissions::from_mode(0o700))?;
    }

    let dir = output_dir.to_path_buf();
    let thread_stop = stop_flag.clone();

    let handle = std::thread::spawn(move || {
        let mut index: u32 = 0;
        let start = std::time::Instant::now();

        tracing::info!(
            dir = %dir.display(),
            interval_secs = interval.as_secs(),
            "screen capture started"
        );

        while !thread_stop.load(Ordering::Relaxed) {
            let elapsed = start.elapsed().as_secs();
            let filename = format!("screen-{:04}-{:04}s.png", index, elapsed);
            let path = dir.join(&filename);

            if let Err(e) = capture_screenshot(&path) {
                tracing::warn!("screen capture failed: {}", e);
                // Don't break — transient failures (e.g., screen locked) are OK
            } else {
                // Set file permissions to 0600 (owner-only)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).ok();
                }
                tracing::debug!(file = %filename, "screen captured");
                index += 1;
            }

            // Sleep in small increments so we can respond to stop quickly
            let sleep_end = std::time::Instant::now() + interval;
            while std::time::Instant::now() < sleep_end {
                if thread_stop.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        }

        tracing::info!(captures = index, "screen capture stopped");
    });

    Ok(ScreenCaptureHandle {
        _thread: Some(handle),
    })
}

/// Capture a single screenshot to the given path.
fn capture_screenshot(path: &Path) -> std::io::Result<()> {
    // macOS: screencapture -x (silent) -C (include cursor) -t png
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("screencapture")
            .args(["-x", "-C", "-t", "png"])
            .arg(path)
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "screencapture failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }

    // Linux: try scrot, fall back to gnome-screenshot
    #[cfg(target_os = "linux")]
    {
        let result = std::process::Command::new("scrot")
            .arg(path)
            .output();

        match result {
            Ok(output) if output.status.success() => {}
            _ => {
                // Fall back to gnome-screenshot
                let output = std::process::Command::new("gnome-screenshot")
                    .args(["--file"])
                    .arg(path)
                    .output()?;

                if !output.status.success() {
                    return Err(std::io::Error::other("no screenshot tool available"));
                }
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        return Err(std::io::Error::other(
            "screen capture not supported on this platform",
        ));
    }

    Ok(())
}

/// Derive the screenshots directory for a given audio recording path.
/// e.g., `/tmp/recording.wav` → `~/.minutes/screens/recording/`
pub fn screens_dir_for(audio_path: &Path) -> PathBuf {
    let stem = audio_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".minutes")
        .join("screens")
        .join(stem)
}

/// List all screenshot files in a directory, sorted by name (chronological).
pub fn list_screenshots(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("png"))
        .collect();

    files.sort();
    files
}

/// Handle that represents an active screen capture session.
/// The capture thread runs until the stop_flag is set.
pub struct ScreenCaptureHandle {
    _thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_screenshots_returns_sorted_pngs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("screen-0002-0060s.png"), "fake").unwrap();
        std::fs::write(dir.path().join("screen-0000-0000s.png"), "fake").unwrap();
        std::fs::write(dir.path().join("screen-0001-0030s.png"), "fake").unwrap();
        std::fs::write(dir.path().join("not-a-screenshot.txt"), "fake").unwrap();

        let files = list_screenshots(dir.path());
        assert_eq!(files.len(), 3);
        assert!(files[0].to_str().unwrap().contains("0000"));
        assert!(files[1].to_str().unwrap().contains("0001"));
        assert!(files[2].to_str().unwrap().contains("0002"));
    }
}
