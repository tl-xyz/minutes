use crate::config::Config;
use crate::error::TranscribeError;
use std::path::Path;
#[cfg(feature = "whisper")]
use std::path::PathBuf;

// Re-export from whisper-guard for public API compatibility
pub use whisper_guard::audio::{normalize_audio, resample, strip_silence};
#[cfg(feature = "whisper")]
pub use whisper_guard::params::{default_whisper_params, streaming_whisper_params};
pub use whisper_guard::segments::{clean_transcript, CleanStats};

// ──────────────────────────────────────────────────────────────
// Transcription pipeline:
//
//   Input audio (.wav, .m4a, .mp3, .ogg)
//        │
//        ├─ .wav ──────────────────────────────────▶ whisper-rs
//        │
//        └─ .m4a/.mp3/.ogg ─▶ symphonia decode ─▶ whisper-rs
//                              (to 16kHz mono PCM)
//
// whisper-rs wraps whisper.cpp, uses Apple Accelerate on M-series.
// Model must be downloaded first via `minutes setup`.
// ──────────────────────────────────────────────────────────────

/// Transcribe an audio file to text.
///
/// With the `whisper` feature (default): uses whisper.cpp via whisper-rs.
/// Without: returns a placeholder transcript (for testing without a model).
///
/// Handles format conversion (m4a/mp3/ogg → PCM) automatically via symphonia.
pub fn transcribe(audio_path: &Path, config: &Config) -> Result<String, TranscribeError> {
    // Step 1: Load audio as 16kHz mono f32 PCM samples
    let samples = load_audio_samples(audio_path)?;

    if samples.is_empty() {
        return Err(TranscribeError::EmptyAudio);
    }

    // Step 1b: Noise reduction (requires denoise feature + config enabled)
    #[cfg(feature = "denoise")]
    let samples = if config.transcription.noise_reduction {
        denoise_audio(&samples, 16000)
    } else {
        samples
    };

    // Step 2: Silence handling.
    // If Silero VAD model is available, whisper handles silence internally via
    // integrated VAD (set in default_whisper_params). Otherwise, fall back to
    // energy-based silence stripping to prevent hallucination loops (issue #21).
    #[cfg(feature = "whisper")]
    let use_integrated_vad = resolve_vad_model_path(config).is_some();
    #[cfg(not(feature = "whisper"))]
    let use_integrated_vad = false;

    let samples = if use_integrated_vad {
        tracing::debug!("Silero VAD available — skipping energy-based silence stripping");
        samples
    } else {
        strip_silence(&samples, 16000)
    };

    if samples.is_empty() {
        return Err(TranscribeError::EmptyAudio);
    }

    // Step 3: Transcribe
    #[cfg(feature = "whisper")]
    {
        transcribe_with_whisper(&samples, audio_path, config)
    }

    #[cfg(not(feature = "whisper"))]
    {
        let _ = config; // suppress unused warning
        let duration_secs = samples.len() as f64 / 16000.0;
        Ok(format!(
            "[Transcription placeholder — whisper feature not enabled]\n\
             Audio file: {}\n\
             Duration: {:.1}s ({} samples at 16kHz)\n\
             \n\
             Build with `cargo build --features whisper` and download a model\n\
             via `minutes setup` to enable real transcription.",
            audio_path.display(),
            duration_secs,
            samples.len(),
        ))
    }
}

/// Real transcription using whisper.cpp via whisper-rs.
#[cfg(feature = "whisper")]
fn transcribe_with_whisper(
    samples: &[f32],
    _audio_path: &Path,
    config: &Config,
) -> Result<String, TranscribeError> {
    // Load whisper model
    let model_path = resolve_model_path(config)?;
    tracing::info!(model = %model_path.display(), "loading whisper model");

    let ctx = whisper_rs::WhisperContext::new_with_params(
        model_path
            .to_str()
            .ok_or_else(|| TranscribeError::ModelLoadError("invalid model path encoding".into()))?,
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|e| TranscribeError::ModelLoadError(format!("{}", e)))?;

    tracing::info!(
        samples = samples.len(),
        duration_secs = samples.len() as f64 / 16000.0,
        "starting whisper transcription"
    );

    let mut state = ctx
        .create_state()
        .map_err(|e| TranscribeError::TranscriptionFailed(format!("create state: {}", e)))?;

    // Resolve VAD model path and convert to string for FullParams lifetime
    let vad_path = resolve_vad_model_path(config);
    let vad_path_str = vad_path.as_ref().and_then(|p| p.to_str());
    let mut params = default_whisper_params(vad_path_str);
    params.set_n_threads(num_cpus());
    params.set_language(config.transcription.language.as_deref());
    params.set_token_timestamps(true);

    state
        .full(params, samples)
        .map_err(|e| TranscribeError::TranscriptionFailed(format!("{}", e)))?;

    let num_segments = state.full_n_segments();

    // Collect segments, filtering by no_speech probability
    let mut lines: Vec<String> = Vec::new();
    let mut skipped_no_speech = 0u32;
    for i in 0..num_segments {
        let segment = match state.get_segment(i) {
            Some(seg) => seg,
            None => continue,
        };

        // Layer 3: Skip segments with high no_speech probability (likely hallucination)
        let no_speech_prob = segment.no_speech_probability();
        if no_speech_prob > 0.8 {
            skipped_no_speech += 1;
            tracing::debug!(
                segment = i,
                no_speech_prob = format!("{:.2}", no_speech_prob),
                "skipping segment — high no_speech probability"
            );
            continue;
        }

        let start_ts = segment.start_timestamp();
        let text = segment
            .to_str_lossy()
            .map_err(|e| TranscribeError::TranscriptionFailed(format!("get text: {}", e)))?;

        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        let mins = start_ts / 6000;
        let secs = (start_ts % 6000) / 100;
        lines.push(format!("[{}:{:02}] {}", mins, secs, text));
    }

    if skipped_no_speech > 0 {
        tracing::info!(
            skipped = skipped_no_speech,
            "filtered segments with high no_speech probability"
        );
    }

    // Layer 2: Remove repetition loops — detect consecutive near-identical segments
    let lines = dedup_segments(lines);

    // Layer 4: Remove interleaved repetition (A/B/A/B patterns, filler-separated loops)
    let lines = dedup_interleaved(lines);

    // Layer 5: Trim trailing noise ([music], [BLANK_AUDIO]) from the end
    let lines = trim_trailing_noise(lines);

    let transcript = lines.join("\n");
    let transcript = if transcript.is_empty() {
        transcript
    } else {
        format!("{}\n", transcript)
    };

    let word_count = transcript.split_whitespace().count();
    tracing::info!(
        segments = num_segments,
        words = word_count,
        "transcription complete"
    );

    Ok(transcript)
}

/// Load audio from any supported format as 16kHz mono f32 samples.
///
/// For non-WAV formats (m4a, mp3, ogg, etc.), prefers ffmpeg when available
/// because symphonia's AAC decoder produces samples that cause whisper to
/// hallucinate on non-English audio (issue #21). Falls back to symphonia
/// when ffmpeg is not installed.
fn load_audio_samples(path: &Path) -> Result<Vec<f32>, TranscribeError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "wav" => load_wav(path),
        "m4a" | "mp3" | "ogg" | "webm" | "mp4" | "aac" => {
            // Prefer ffmpeg — its resampler and AAC decoder produce samples that
            // whisper transcribes correctly across all languages. Symphonia's AAC
            // decoder produces subtly different samples that trigger hallucination
            // loops on non-English audio (confirmed in issue #21).
            match decode_with_ffmpeg(path) {
                Ok(samples) => Ok(samples),
                Err(e) => {
                    let is_not_found = e.to_string().contains("not available")
                        || e.to_string().contains("not found");
                    if is_not_found {
                        tracing::warn!(
                            "ffmpeg not found — falling back to symphonia for {} decoding. \
                             Non-English audio may produce poor results. \
                             Install ffmpeg: brew install ffmpeg (macOS) / apt install ffmpeg (Linux)",
                            ext
                        );
                    } else {
                        tracing::warn!(
                            error = %e,
                            "ffmpeg decode failed — falling back to symphonia"
                        );
                    }
                    decode_with_symphonia(path)
                }
            }
        }
        other => Err(TranscribeError::UnsupportedFormat(other.to_string())),
    }
}

/// Load WAV file as f32 samples, converting to 16kHz mono if needed.
fn load_wav(path: &Path) -> Result<Vec<f32>, TranscribeError> {
    let reader = hound::WavReader::open(path).map_err(|e| {
        if e.to_string().contains("Not a WAVE file") || e.to_string().contains("unexpected EOF") {
            TranscribeError::UnsupportedFormat("corrupt or invalid WAV file".into())
        } else {
            TranscribeError::Io(std::io::Error::other(e.to_string()))
        }
    })?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;

    // Read all samples as f32, normalizing by actual bit depth
    let bits = spec.bits_per_sample;
    let max_val = (1_i64 << (bits - 1)) as f32; // e.g. 16-bit → 32768.0
    let raw_samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => reader
            .into_samples::<i32>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / max_val)
            .collect(),
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
    };

    if raw_samples.is_empty() {
        return Err(TranscribeError::EmptyAudio);
    }

    // Convert to mono
    let mono = if channels > 1 {
        raw_samples
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect()
    } else {
        raw_samples
    };

    // Resample to 16kHz if needed
    let resampled = if sample_rate != 16000 {
        resample(&mono, sample_rate, 16000)
    } else {
        mono
    };

    // Auto-normalize: if peak is below target, boost so whisper gets usable levels.
    // Quiet mics (e.g. MacBook Pro) can produce peaks of 0.004 which whisper can't detect.
    Ok(normalize_audio(&resampled))
}

/// Decode audio with ffmpeg (preferred for non-WAV formats).
///
/// Shells out to `ffmpeg` to convert any audio to 16kHz mono f32le PCM.
/// This matches exactly what whisper-cli does and produces samples that
/// whisper transcribes correctly across all languages.
///
/// Returns an error if ffmpeg is not installed or the conversion fails,
/// allowing the caller to fall back to symphonia.
fn decode_with_ffmpeg(path: &Path) -> Result<Vec<f32>, TranscribeError> {
    use std::process::Command;

    let tmp_dir = std::env::temp_dir();
    let tmp_wav = tmp_dir.join(format!("minutes-ffmpeg-{}.wav", std::process::id()));

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            path.to_str().unwrap_or(""),
            "-ar",
            "16000", // 16kHz sample rate
            "-ac",
            "1", // mono
            "-f",
            "wav", // WAV output
            "-y",  // overwrite
        ])
        .arg(&tmp_wav)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| {
            TranscribeError::TranscriptionFailed(format!("ffmpeg not available: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Clean up temp file on failure
        let _ = std::fs::remove_file(&tmp_wav);
        return Err(TranscribeError::TranscriptionFailed(format!(
            "ffmpeg conversion failed: {}",
            stderr.lines().last().unwrap_or("unknown error")
        )));
    }

    tracing::info!(
        source = %path.display(),
        "decoded audio with ffmpeg (16kHz mono WAV)"
    );

    // Load the ffmpeg-produced WAV (already 16kHz mono)
    let result = load_wav(&tmp_wav);

    // Clean up temp file
    let _ = std::fs::remove_file(&tmp_wav);

    result
}

/// Decode audio with symphonia (handles m4a, mp3, ogg, etc.)
/// Outputs 16kHz mono f32 samples.
fn decode_with_symphonia(path: &Path) -> Result<Vec<f32>, TranscribeError> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| TranscribeError::UnsupportedFormat(format!("probe failed: {}", e)))?;

    let mut format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| TranscribeError::UnsupportedFormat("no audio track found".into()))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);

    let decoder_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|e| TranscribeError::UnsupportedFormat(format!("decoder: {}", e)))?;

    let mut all_samples: Vec<f32> = Vec::new();

    // Decode all packets
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break; // End of stream
            }
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(_) => continue, // Skip bad packets
        };

        let spec = *decoded.spec();
        let duration = decoded.capacity();

        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();

        // Convert to mono if needed
        if channels > 1 {
            for chunk in samples.chunks(channels) {
                let mono_sample = chunk.iter().sum::<f32>() / channels as f32;
                all_samples.push(mono_sample);
            }
        } else {
            all_samples.extend_from_slice(samples);
        }
    }

    if all_samples.is_empty() {
        return Err(TranscribeError::EmptyAudio);
    }

    // Resample to 16kHz if needed
    let resampled = if sample_rate != 16000 {
        resample(&all_samples, sample_rate, 16000)
    } else {
        all_samples
    };

    Ok(normalize_audio(&resampled))
}

// resample() and normalize_audio() are provided by whisper_guard::audio
// and re-exported at the top of this file.

// Segment cleaning functions (dedup_segments, dedup_interleaved, trim_trailing_noise,
// clean_transcript, CleanStats) are provided by whisper_guard::segments.
// They are re-exported as pub use at the top of this file for API compatibility.
// The private wrappers below delegate to whisper-guard so internal callers
// (transcribe_with_whisper) continue working without path changes.
#[cfg(feature = "whisper")]
use whisper_guard::segments as wg_segments;

// Thin delegates to whisper-guard (only called by transcribe_with_whisper behind cfg(whisper))
#[cfg(feature = "whisper")]
fn dedup_segments(lines: Vec<String>) -> Vec<String> {
    wg_segments::dedup_segments(&lines)
}
#[cfg(feature = "whisper")]
fn dedup_interleaved(lines: Vec<String>) -> Vec<String> {
    wg_segments::dedup_interleaved(&lines)
}
#[cfg(feature = "whisper")]
fn trim_trailing_noise(lines: Vec<String>) -> Vec<String> {
    wg_segments::trim_trailing_noise(&lines)
}

// ── Noise reduction ──────────────────────────────────────────

/// Apply RNNoise-based noise reduction to audio samples.
///
/// nnnoiseless requires 48kHz f32 audio in 480-sample frames with values
/// in i16 range (-32768 to 32767). This function handles resampling to/from
/// 48kHz and the scaling automatically.
///
/// Primes the DenoiseState with a silence frame to avoid first-frame
/// fade-in artifacts.
#[cfg(feature = "denoise")]
fn denoise_audio(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    use nnnoiseless::{DenoiseState, FRAME_SIZE};

    if samples.is_empty() {
        return samples.to_vec();
    }

    // Resample to 48kHz if needed (nnnoiseless requires exactly 48kHz)
    let (samples_48k, original_rate) = if sample_rate != 48000 {
        (resample(samples, sample_rate, 48000), Some(sample_rate))
    } else {
        (samples.to_vec(), None)
    };

    // Scale to i16 range as nnnoiseless expects
    let scaled: Vec<f32> = samples_48k.iter().map(|s| s * 32767.0).collect();

    let mut state = DenoiseState::new();
    let mut output = Vec::with_capacity(scaled.len());
    let mut frame_out = [0.0f32; FRAME_SIZE];

    // Prime with a silence frame to avoid first-frame fade-in artifact
    let silence = [0.0f32; FRAME_SIZE];
    state.process_frame(&mut frame_out, &silence);

    for chunk in scaled.chunks(FRAME_SIZE) {
        if chunk.len() == FRAME_SIZE {
            state.process_frame(&mut frame_out, chunk);
            output.extend_from_slice(&frame_out);
        } else {
            // Pad last frame with zeros
            let mut padded = [0.0f32; FRAME_SIZE];
            padded[..chunk.len()].copy_from_slice(chunk);
            state.process_frame(&mut frame_out, &padded);
            output.extend_from_slice(&frame_out[..chunk.len()]);
        }
    }

    // Scale back to -1.0..1.0 range
    let denoised: Vec<f32> = output.iter().map(|s| s / 32767.0).collect();

    // Resample back to original rate if we upsampled
    let denoised = if let Some(orig) = original_rate {
        resample(&denoised, 48000, orig)
    } else {
        denoised
    };

    let original_rms: f32 =
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    let denoised_rms: f32 =
        (denoised.iter().map(|s| s * s).sum::<f32>() / denoised.len() as f32).sqrt();

    tracing::info!(
        original_rms = format!("{:.4}", original_rms),
        denoised_rms = format!("{:.4}", denoised_rms),
        reduction_db = format!(
            "{:.1}",
            20.0 * (denoised_rms / original_rms.max(0.0001)).log10()
        ),
        "noise reduction applied"
    );

    denoised
}

/// Resolve the whisper model file path for dictation (uses dictation.model config).
#[cfg(feature = "whisper")]
pub fn resolve_model_path_for_dictation(config: &Config) -> Result<PathBuf, TranscribeError> {
    let model_name = &config.dictation.model;
    let model_dir = &config.transcription.model_path;

    let candidates = [
        model_dir.join(format!("ggml-{}.bin", model_name)),
        model_dir.join(format!("whisper-{}.bin", model_name)),
        model_dir.join(format!("{}.bin", model_name)),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    let direct = PathBuf::from(model_name);
    if direct.exists() {
        return Ok(direct);
    }

    Err(TranscribeError::ModelNotFound(format!(
        "Expected model file \"ggml-{}.bin\" in {}",
        model_name,
        model_dir.display(),
    )))
}

/// Resolve the whisper model file path.
#[cfg(feature = "whisper")]
fn resolve_model_path(config: &Config) -> Result<PathBuf, TranscribeError> {
    let model_name = &config.transcription.model;
    let model_dir = &config.transcription.model_path;

    // Try common naming patterns
    let candidates = [
        model_dir.join(format!("ggml-{}.bin", model_name)),
        model_dir.join(format!("whisper-{}.bin", model_name)),
        model_dir.join(format!("{}.bin", model_name)),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    // If model_name is an absolute path, try it directly
    let direct = PathBuf::from(model_name);
    if direct.exists() {
        return Ok(direct);
    }

    Err(TranscribeError::ModelNotFound(format!(
        "Expected model file \"ggml-{}.bin\" in {}",
        model_name,
        model_dir.display(),
    )))
}

/// Resolve the Silero VAD model path. Returns None if VAD is disabled or model not found.
#[cfg(feature = "whisper")]
fn resolve_vad_model_path(config: &Config) -> Option<PathBuf> {
    let vad_model = &config.transcription.vad_model;
    if vad_model.is_empty() {
        return None;
    }

    let model_dir = &config.transcription.model_path;
    let mut candidates = vec![
        model_dir.join(format!("ggml-{}.bin", vad_model)),
        model_dir.join(format!("{}.bin", vad_model)),
    ];
    // Fallback: accept old "ggml-silero-vad.bin" name for backward compatibility,
    // but only when the config is using a silero-variant name (the default).
    if vad_model.starts_with("silero") {
        candidates.push(model_dir.join("ggml-silero-vad.bin"));
    }

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    // Try as absolute path
    let direct = PathBuf::from(vad_model);
    if direct.exists() {
        return Some(direct);
    }

    tracing::debug!(
        vad_model = vad_model,
        "VAD model not found — falling back to energy-based silence stripping"
    );
    None
}

// default_whisper_params, streaming_whisper_params, and num_cpus
// are re-exported from whisper_guard::params via `pub use` at the top of this file.
#[cfg(feature = "whisper")]
fn num_cpus() -> i32 {
    whisper_guard::params::num_cpus()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "whisper")]
    fn resolve_model_path_returns_error_for_missing() {
        let config = Config {
            transcription: crate::config::TranscriptionConfig {
                model: "nonexistent".into(),
                model_path: PathBuf::from("/tmp/no-such-dir"),
                min_words: 10,
                language: Some("en".into()),
                vad_model: String::new(),
                noise_reduction: false,
            },
            ..Config::default()
        };
        let result = resolve_model_path(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("minutes setup --model tiny"),
            "error should tell user how to fix it: {}",
            err
        );
        assert!(
            err.contains("ggml-nonexistent.bin"),
            "error should include expected model filename: {}",
            err
        );
        assert!(
            err.contains("/tmp/no-such-dir"),
            "error should include the model directory: {}",
            err
        );
    }

    #[test]
    fn load_wav_rejects_empty_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("empty.wav");
        std::fs::write(&path, "").unwrap();
        let result = load_wav(&path);
        assert!(result.is_err());
    }

    #[test]
    fn load_wav_reads_valid_wav() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.wav");

        // Create a short WAV with hound
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for i in 0..16000 {
            let sample =
                (10000.0 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin()) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();

        let samples = load_wav(&path).unwrap();
        assert!(!samples.is_empty());
        // 1 second at 16kHz = 16000 samples
        assert_eq!(samples.len(), 16000);
    }

    #[test]
    fn load_audio_rejects_unknown_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.xyz");
        std::fs::write(&path, "not audio").unwrap();
        let result = load_audio_samples(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("xyz"));
    }
}
