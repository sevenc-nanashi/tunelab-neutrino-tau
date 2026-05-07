use crate::config;
use anyhow::Context;
use itertools::Itertools;
use log::{debug, error, info};
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct Engine {
    dll_path: PathBuf,
    neutrino_path: std::path::PathBuf,
    server: Option<std::process::Child>,
}

type WavData = (wav_io::header::WavHeader, Vec<f32>);
const CLIENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
const CANCEL_GRACE_PERIOD: Duration = Duration::from_secs(3);
const SERVER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const LOG_FILE_NAME: &str = "neutrino-tau-native.log";
static LOGGER_INIT: std::sync::Once = std::sync::Once::new();

const NEUTRINO_FILENAME: &str = cfg_select! {
    windows => "neutrino.exe",
    _ => "neutrino",
};
const NEUTRINO_SERVER_FILENAME: &str = cfg_select! {
    windows => "neutrino_server.exe",
    _ => "neutrino_server",
};
const NEUTRINO_CLIENT_FILENAME: &str = cfg_select! {
    windows => "neutrino_client.exe",
    _ => "neutrino_client",
};

impl Engine {
    pub fn new(dll_path: std::path::PathBuf) -> anyhow::Result<Self> {
        init_logger(&dll_path);
        info!("Creating engine. dll_dir={}", dll_path.display());
        let config_path = dll_path.join("config.json");
        info!("Loading config from {}", config_path.display());
        let mut config = if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)
                .inspect_err(|e| {
                    error!("Failed to read config file: {}", e);
                })
                .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
            let config = serde_json::from_str(&config_str)
                .inspect_err(|e| {
                    error!("Failed to parse config file: {}", e);
                })
                .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
            info!("Loaded config.json successfully");
            config
        } else {
            info!("config.json not found. Using default config");
            config::Config::default()
        };
        if config.neutrino_path.is_none() {
            let selected_neutrino_path = Self::select_neutrino_path()?;
            config.neutrino_path = Some(
                selected_neutrino_path
                    .to_str()
                    .context("Failed to convert Neutrino path to string, try moving Neutrino to a path with ASCII characters only")?
                    .to_string(),
            );

            std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
                .inspect_err(|e| {
                    error!("Failed to write config file: {}", e);
                })
                .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;
            info!("Persisted config.json successfully");
        }

        let neutrino_path = config.neutrino_path.as_ref().unwrap();
        if !std::path::Path::new(neutrino_path).exists() {
            error!("Neutrino path does not exist: {}", neutrino_path);
            return Err(anyhow::anyhow!(
                "Neutrino path does not exist: {}",
                neutrino_path
            ));
        }
        info!("Using neutrino_path={}", neutrino_path);

        Ok(Self {
            dll_path,
            neutrino_path: config.neutrino_path.unwrap().into(),
            server: None,
        })
    }

    fn select_neutrino_path() -> anyhow::Result<PathBuf> {
        info!("neutrino_path is not configured. Opening file dialog");
        let Some(result) = native_dialog::FileDialogBuilder::default()
            .set_title("Select neutrino.exe")
            .add_filter("Executable", ["exe"])
            .open_single_file()
            .show()?
        else {
            error!("Neutrino path is required but not provided");
            return Err(anyhow::anyhow!(
                "Neutrino path is required but not provided"
            ));
        };

        info!("Selected neutrino.exe candidate: {}", result.display());
        if !result.exists() {
            error!(
                "Selected Neutrino path does not exist: {}",
                result.display()
            );
            return Err(anyhow::anyhow!(
                "Selected Neutrino path does not exist: {}",
                result.display()
            ));
        }
        if result.file_name().and_then(|n| n.to_str()) != Some("neutrino.exe") {
            error!("Selected file is not neutrino.exe: {}", result.display());
            return Err(anyhow::anyhow!(
                "Selected file is not neutrino.exe: {}",
                result.display()
            ));
        }

        let neutrino_root = result.parent().and_then(|p| p.parent()).ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to determine Neutrino root directory from selected path: {}",
                result.display()
            )
        })?;
        info!("Resolved neutrino root: {}", neutrino_root.display());
        Ok(neutrino_root.to_path_buf())
    }

    fn spawn_server(&mut self) -> anyhow::Result<()> {
        if self
            .server
            .as_mut()
            .is_some_and(|s| s.try_wait().map(|w| w.is_none()).unwrap_or(false))
        {
            info!("Neutrino server is already running");
            return Ok(());
        }
        let server_path = self.neutrino_path.join("bin").join("neutrino_server.exe");
        if !server_path.exists() {
            error!(
                "Neutrino server executable not found at: {}",
                server_path.display()
            );
            return Err(anyhow::anyhow!(
                "Neutrino server executable not found at: {}",
                server_path.display()
            ));
        }
        info!("Spawning Neutrino server: {}", server_path.display());

        let child = std::process::Command::new(server_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .inspect_err(|e| {
                error!("Failed to spawn Neutrino server: {}", e);
            })
            .map_err(|e| anyhow::anyhow!("Failed to spawn Neutrino server: {}", e))?;

        self.server = Some(child);
        info!("Neutrino server spawned successfully");
        Ok(())
    }

    pub fn load_voices(&self) -> anyhow::Result<Vec<crate::speaker::VoiceSource>> {
        info!("Loading voice sources");
        let mut speakers = Vec::new();
        let models_path = self.neutrino_path.join("model");
        if !models_path.exists() {
            error!("Neutrino model directory not found");
            return Err(anyhow::anyhow!("Neutrino model directory not found"));
        }

        for entry in std::fs::read_dir(models_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                match crate::speaker::VoiceSource::load(&entry.path()) {
                    Ok(voice) => speakers.push(voice),
                    Err(e) => {
                        error!(
                            "Failed to load voice from {}: {}",
                            entry.path().display(),
                            e
                        );
                    }
                }
            }
        }

        info!("Loaded {} voice sources", speakers.len());
        Ok(speakers)
    }

    pub fn synthesize(
        &mut self,
        synthesis_task_json: &str,
        cancel_token: &Arc<AtomicBool>,
    ) -> anyhow::Result<String> {
        self.log_info(&format!(
            "Starting synthesis. payload_bytes={}",
            synthesis_task_json.len()
        ));
        let (payload, score, tunelab_start_in_synthesis_time) =
            Self::prepare_synthesis_input(synthesis_task_json).inspect_err(|e| {
                self.log_error(&format!("Failed to prepare synthesis input: {}", e));
            })?;

        let timings = self.synthesize_timing(&payload.voice_id, &score, cancel_token)?;
        let mapped_phoneme_groups = self.map_phonemes_to_notes(&score, &timings)?;
        let merged_phonemes = Self::merge_phonemes_with_payload(
            &payload,
            &mapped_phoneme_groups,
            tunelab_start_in_synthesis_time,
        );

        let style_score = Self::transpose_score_pitches(&score, payload.style_shift);
        let inferred_f0_values = self.synthesize_f0(
            &payload.voice_id,
            &style_score,
            &merged_phonemes,
            cancel_token,
        )?;
        // Infer f0 on style-shifted notes, then shift f0 back to the original key.
        let f0_values = Self::shift_f0_by_semitones(&inferred_f0_values, -payload.style_shift);

        let mapped_f0_values = Self::apply_payload_pitch_to_f0(
            &payload.pitch,
            &f0_values,
            tunelab_start_in_synthesis_time,
        );
        let shifted_mapped_f0_values =
            Self::shift_f0_by_cents(&mapped_f0_values, payload.pitch_shift_cents);

        let waveform_score =
            Self::transpose_score_pitches(&style_score, payload.waveform_style_shift);
        let wav_data = self.synthesize_waveform(
            &payload.voice_id,
            &waveform_score,
            &merged_phonemes,
            &shifted_mapped_f0_values,
            cancel_token,
        )?;
        let response = Self::build_synthesis_response(
            &payload,
            &shifted_mapped_f0_values,
            &mapped_phoneme_groups,
            &merged_phonemes,
            wav_data,
            tunelab_start_in_synthesis_time,
        );

        let response_json = serde_json::to_string(&response).inspect_err(|e| {
            error!("Failed to serialize synthesis response: {}", e);
        })?;
        info!("Synthesis completed successfully");
        Ok(response_json)
    }

    fn build_log_path(dll_path: &Path) -> PathBuf {
        dll_path.join(LOG_FILE_NAME)
    }

    pub fn log_info(&self, message: &str) {
        info!("{}", message);
    }

    pub fn log_error(&self, message: &str) {
        error!("{}", message);
    }

    fn transpose_score_pitches(
        score: &crate::neutrino_score::Score,
        semitones: f64,
    ) -> crate::neutrino_score::Score {
        if !semitones.is_finite() || semitones.abs() < f64::EPSILON {
            return score.clone();
        }
        let delta = semitones.round() as i32;
        if delta == 0 {
            return score.clone();
        }

        let mut transposed = score.clone();
        for note in &mut transposed.notes {
            note.pitch = note.pitch.map(|p| {
                let shifted = (p as i32 + delta).clamp(0, 127);
                shifted as u8
            });
        }
        transposed
    }

    fn shift_f0_by_semitones(f0_values: &[f32], semitones: f64) -> Vec<f32> {
        if !semitones.is_finite() || semitones.abs() < f64::EPSILON {
            return f0_values.to_vec();
        }
        let ratio = 2.0_f32.powf((semitones as f32) / 12.0);
        f0_values
            .iter()
            .map(|&f0| {
                if f0.is_finite() && f0 > 0.0 {
                    f0 * ratio
                } else {
                    f0
                }
            })
            .collect()
    }

    fn shift_f0_by_cents(f0_values: &[f32], cents: f64) -> Vec<f32> {
        if !cents.is_finite() || cents.abs() < f64::EPSILON {
            return f0_values.to_vec();
        }
        Self::shift_f0_by_semitones(f0_values, cents / 100.0)
    }

    fn prepare_synthesis_input(
        synthesis_task_json: &str,
    ) -> anyhow::Result<(
        crate::synthesizer::SynthesisTaskPayload,
        crate::neutrino_score::Score,
        f64,
    )> {
        let payload =
            serde_json::from_str::<crate::synthesizer::SynthesisTaskPayload>(synthesis_task_json)
                .map_err(|e| anyhow::anyhow!("Failed to parse synthesis task payload: {}", e))?;
        let score = crate::synthesizer::task_notes_to_score(&payload.notes)?;
        let tunelab_start_in_synthesis_time =
            (score.notes[1].start_time_ns as f64 / 1e9) - payload.notes[0].start_time;
        Ok((payload, score, tunelab_start_in_synthesis_time))
    }

    fn merge_phonemes_with_payload(
        payload: &crate::synthesizer::SynthesisTaskPayload,
        mapped_phoneme_groups: &[Vec<crate::synthesizer::TimingLabel>],
        tunelab_start_in_synthesis_time: f64,
    ) -> Vec<crate::synthesizer::TimingLabel> {
        let mut merged_phonemes =
            Vec::with_capacity(payload.notes.iter().map(|n| n.phonemes.len()).sum());

        // NOTE: pauが最初と最後にあるのでNoneではさむ
        for (synthesized_phonemes, note) in mapped_phoneme_groups.iter().zip(
            std::iter::once(None)
                .chain(payload.notes.iter().map(Some))
                .chain(std::iter::once(None)),
        ) {
            match note {
                Some(note) => {
                    if note.phonemes.len() != synthesized_phonemes.len() {
                        merged_phonemes.extend(synthesized_phonemes.iter().cloned());
                    } else {
                        for phoneme in &note.phonemes {
                            merged_phonemes.push(crate::synthesizer::TimingLabel {
                                start_time_ns: ((tunelab_start_in_synthesis_time
                                    + phoneme.start_time)
                                    * 1e9) as u64,
                                end_time_ns: ((tunelab_start_in_synthesis_time + phoneme.end_time)
                                    * 1e9) as u64,
                                phoneme: phoneme.symbol.clone(),
                            });
                        }
                    }
                }
                None => merged_phonemes.extend(synthesized_phonemes.iter().cloned()),
            }
        }

        merged_phonemes
    }

    fn apply_payload_pitch_to_f0(
        pitch: &crate::synthesizer::PitchPayload,
        f0_values: &[f32],
        tunelab_start_in_synthesis_time: f64,
    ) -> Vec<f32> {
        const F0_FRAME_RATE_HZ: f64 = 99.84;
        let mut mapped_f0_values = f0_values.to_vec();

        // NOTE: f0 frame = 99.84 Hz
        for ((time_before, midi_before), (time_after, midi_after)) in
            pitch.times.iter().zip(pitch.values.iter()).tuple_windows()
        {
            if !midi_before.is_finite() || !midi_after.is_finite() {
                continue;
            }
            let before_time_in_synthesis = *time_before + tunelab_start_in_synthesis_time;
            let next_time_in_synthesis = *time_after + tunelab_start_in_synthesis_time;
            if next_time_in_synthesis <= before_time_in_synthesis {
                continue;
            }
            let first_frame = (before_time_in_synthesis * F0_FRAME_RATE_HZ).ceil() as i64;
            let last_frame = (next_time_in_synthesis * F0_FRAME_RATE_HZ).floor() as i64;
            let frame_iter: Box<dyn Iterator<Item = i64>> = if first_frame <= last_frame {
                Box::new(first_frame..=last_frame)
            } else {
                let nearest =
                    ((before_time_in_synthesis + next_time_in_synthesis) * 0.5 * F0_FRAME_RATE_HZ)
                        .round() as i64;
                Box::new(std::iter::once(nearest))
            };
            for frame in frame_iter {
                if frame < 0 {
                    continue;
                }
                let frame_time = frame as f64 / F0_FRAME_RATE_HZ;
                let t = (frame_time - before_time_in_synthesis)
                    / (next_time_in_synthesis - before_time_in_synthesis);
                let interpolated_midi = midi_before.0 + t * (midi_after.0 - midi_before.0);
                let index = frame as usize;
                let f0_value = crate::synthesizer::midi_to_freq(interpolated_midi as f32);
                if index < mapped_f0_values.len() {
                    mapped_f0_values[index] = f0_value;
                }
            }
        }

        mapped_f0_values
    }

    fn build_note_phonemes(
        mapped_phoneme_groups: &[Vec<crate::synthesizer::TimingLabel>],
        merged_phonemes: &[crate::synthesizer::TimingLabel],
        tunelab_start_in_synthesis_time: f64,
    ) -> Vec<crate::synthesizer::NotePhonemes> {
        let mut merged_iter = merged_phonemes.iter();
        mapped_phoneme_groups
            .iter()
            .enumerate()
            .filter_map(|(i, group)| {
                let current_group = group
                    .iter()
                    .filter_map(|_| merged_iter.next())
                    .collect::<Vec<_>>();
                if i == 0 || i == mapped_phoneme_groups.len() - 1 {
                    // 最初と最後のグループはpauなのでスキップ
                    None
                } else {
                    Some(crate::synthesizer::NotePhonemes {
                        note_index: i - 1,
                        phonemes: current_group
                            .iter()
                            .map(|p| crate::synthesizer::SynthesizedPhoneme {
                                start_time: (p.start_time_ns as f64) / 1e9
                                    - tunelab_start_in_synthesis_time,
                                end_time: (p.end_time_ns as f64) / 1e9
                                    - tunelab_start_in_synthesis_time,
                                symbol: p.phoneme.clone(),
                            })
                            .collect(),
                    })
                }
            })
            .collect()
    }

    fn build_synthesis_response(
        payload: &crate::synthesizer::SynthesisTaskPayload,
        f0_values: &[f32],
        mapped_phoneme_groups: &[Vec<crate::synthesizer::TimingLabel>],
        merged_phonemes: &[crate::synthesizer::TimingLabel],
        wav_data: WavData,
        tunelab_start_in_synthesis_time: f64,
    ) -> crate::synthesizer::SynthesisResponse {
        let pitch_times = (0..f0_values.len())
            .map(|i| (i as f64) / 99.84 - tunelab_start_in_synthesis_time)
            .collect::<Vec<_>>();
        let pitch_values = f0_values
            .iter()
            .map(|&f| crate::synthesizer::freq_to_midi(f) as f64)
            .collect::<Vec<_>>();
        let mut skipped_pitches = pitch_values
            .iter()
            .map(|&midi| !midi.is_finite())
            .collect::<Vec<_>>();

        for (i, (left, current, right)) in pitch_values.iter().tuple_windows().enumerate() {
            if left == current && current == right && !skipped_pitches[i + 1] {
                skipped_pitches[i] = true;
            }
        }

        let mono_samples = if wav_data.0.channels == 1 {
            wav_data.1
        } else {
            wav_data
                .1
                .chunks_exact(wav_data.0.channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / (wav_data.0.channels as f32))
                .collect()
        };

        crate::synthesizer::SynthesisResponse {
            start_time: -tunelab_start_in_synthesis_time,
            sample_rate: wav_data.0.sample_rate as _,
            sample_count: mono_samples.len() as _,
            samples: mono_samples,
            pitch_times: pitch_times
                .iter()
                .zip(skipped_pitches.iter())
                .filter_map(|(&t, &skipped)| if skipped { None } else { Some(t) })
                .collect(),
            pitch_values: pitch_values
                .iter()
                .zip(skipped_pitches.iter())
                .filter_map(|(&midi, &skipped)| if skipped { None } else { Some(midi) })
                .collect(),
            note_phonemes: Self::build_note_phonemes(
                mapped_phoneme_groups,
                merged_phonemes,
                tunelab_start_in_synthesis_time,
            ),
            note_count: payload.notes.len(),
            phoneme_count: merged_phonemes.len(),
            property_count: 0, // 今のところプロパティは返さない
        }
    }

    fn synthesize_timing(
        &mut self,
        voice_id: &str,
        score: &crate::neutrino_score::Score,
        cancel_token: &Arc<AtomicBool>,
    ) -> anyhow::Result<Vec<crate::synthesizer::TimingLabel>> {
        let label_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary label file: {}", e))?;
        let label_path = label_file.path().to_string_lossy().to_string();
        for label in crate::neutrino_score::compose_labels_from_score(score)? {
            // HTS label timing uses 100ns units.
            let start_time_100ns = label.start_time_ns / 100;
            let end_time_100ns = label.end_time_ns / 100;
            writeln!(
                &label_file,
                "{} {} {}",
                start_time_100ns, end_time_100ns, label.label,
            )
            .map_err(|e| anyhow::anyhow!("Failed to write to label file: {}", e))?;
        }
        let generated_label_file = tempfile::NamedTempFile::new().map_err(|e| {
            anyhow::anyhow!("Failed to create temporary generated label file: {}", e)
        })?;
        let generated_label_path = generated_label_file.path().to_string_lossy().to_string();
        let melspec_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary melspec file: {}", e))?;
        let melspec_path = melspec_file.path().to_string_lossy().to_string();
        let f0_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary f0 file: {}", e))?;
        let f0_path = f0_file.path().to_string_lossy().to_string();
        let generated_wav_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary wav file: {}", e))?;
        let generated_wav_path = generated_wav_file.path().to_string_lossy().to_string();
        let model_dir = self.neutrino_path.join("model").join(voice_id);
        let model_dir = model_dir.to_string_lossy().to_string();
        let cpu_count = num_cpus::get().to_string();
        self.invoke_client(
            &[
                label_path.as_str(),
                generated_label_path.as_str(),
                f0_path.as_str(),
                melspec_path.as_str(),
                generated_wav_path.as_str(),
                model_dir.as_str(),
                "-n",
                cpu_count.as_str(),
                "-m",
                "-t",
                "--skip-melspec",
                "--skip-f0",
                "--skip-wav",
            ],
            cancel_token,
        )?;
        let label_data = std::fs::read_to_string(generated_label_path)
            .map_err(|e| anyhow::anyhow!("Failed to read generated label file: {}", e))?;
        let labels = crate::synthesizer::parse_timing_label_file(&label_data)?;

        Ok(labels)
    }

    fn map_phonemes_to_notes(
        &self,
        score: &crate::neutrino_score::Score,
        timings: &[crate::synthesizer::TimingLabel],
    ) -> anyhow::Result<Vec<Vec<crate::synthesizer::TimingLabel>>> {
        let mut timing_labels_iter = timings.iter();
        score
            .notes
            .iter()
            .map(|note| {
                let phonemes = note
                    .phonemes
                    .iter()
                    .map(|_| timing_labels_iter.next().ok_or_else(|| {
                        anyhow::anyhow!(
                            "Not enough timing labels for the number of phonemes in the score. Note start time: {} ns",
                            note.start_time_ns
                        )
                    }).cloned())
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(phonemes)
            })
            .collect::<anyhow::Result<Vec<Vec<crate::synthesizer::TimingLabel>>>>()
    }

    fn synthesize_f0(
        &mut self,
        voice_id: &str,
        score: &crate::neutrino_score::Score,
        timings: &[crate::synthesizer::TimingLabel],
        cancel_token: &Arc<AtomicBool>,
    ) -> anyhow::Result<Vec<f32>> {
        let label_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary label file: {}", e))?;
        let label_path = label_file.path().to_string_lossy().to_string();
        for label in crate::neutrino_score::compose_labels_from_score(score)? {
            // HTS label timing uses 100ns units.
            let start_time_100ns = label.start_time_ns / 100;
            let end_time_100ns = label.end_time_ns / 100;
            writeln!(
                &label_file,
                "{} {} {}",
                start_time_100ns, end_time_100ns, label.label,
            )
            .map_err(|e| anyhow::anyhow!("Failed to write to label file: {}", e))?;
        }
        let generated_label_file = tempfile::NamedTempFile::new().map_err(|e| {
            anyhow::anyhow!("Failed to create temporary generated label file: {}", e)
        })?;
        let generated_label_path = generated_label_file.path().to_string_lossy().to_string();
        for label in timings {
            let start_time_100ns = label.start_time_ns / 100;
            let end_time_100ns = label.end_time_ns / 100;
            writeln!(
                &generated_label_file,
                "{} {} {}",
                start_time_100ns, end_time_100ns, label.phoneme,
            )
            .map_err(|e| anyhow::anyhow!("Failed to write to generated label file: {}", e))?;
        }
        let f0_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary f0 file: {}", e))?;
        let f0_path = f0_file.path().to_string_lossy().to_string();
        let melspec_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary melspec file: {}", e))?;
        let melspec_path = melspec_file.path().to_string_lossy().to_string();
        let generated_wav_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary wav file: {}", e))?;
        let generated_wav_path = generated_wav_file.path().to_string_lossy().to_string();
        let model_dir = self.neutrino_path.join("model").join(voice_id);
        let model_dir = model_dir.to_string_lossy().to_string();
        let cpu_count = num_cpus::get().to_string();
        self.invoke_client(
            &[
                label_path.as_str(),
                generated_label_path.as_str(),
                f0_path.as_str(),
                melspec_path.as_str(),
                generated_wav_path.as_str(),
                model_dir.as_str(),
                "-n",
                cpu_count.as_str(),
                "-m",
                "-t",
                "--skip-timing",
                "--skip-melspec",
                "--skip-wav",
            ],
            cancel_token,
        )?;
        let f0_data = std::fs::read(&f0_path)
            .map_err(|e| anyhow::anyhow!("Failed to read generated f0 file: {}", e))?;
        let f0_values = f0_data
            .chunks_exact(4)
            .map(|chunk| {
                let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                f32::from_le_bytes(bytes)
            })
            .collect();
        Ok(f0_values)
    }

    fn synthesize_waveform(
        &mut self,
        voice_id: &str,
        score: &crate::neutrino_score::Score,
        timings: &[crate::synthesizer::TimingLabel],
        f0_values: &[f32],
        cancel_token: &Arc<AtomicBool>,
    ) -> anyhow::Result<WavData> {
        let label_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary label file: {}", e))?;
        let label_path = label_file.path().to_string_lossy().to_string();
        for label in crate::neutrino_score::compose_labels_from_score(score)? {
            // HTS label timing uses 100ns units.
            let start_time_100ns = label.start_time_ns / 100;
            let end_time_100ns = label.end_time_ns / 100;
            writeln!(
                &label_file,
                "{} {} {}",
                start_time_100ns, end_time_100ns, label.label,
            )
            .map_err(|e| anyhow::anyhow!("Failed to write to label file: {}", e))?;
        }
        let generated_label_file = tempfile::NamedTempFile::new().map_err(|e| {
            anyhow::anyhow!("Failed to create temporary generated label file: {}", e)
        })?;
        let generated_label_path = generated_label_file.path().to_string_lossy().to_string();
        for label in timings {
            let start_time_100ns = label.start_time_ns / 100;
            let end_time_100ns = label.end_time_ns / 100;
            writeln!(
                &generated_label_file,
                "{} {} {}",
                start_time_100ns, end_time_100ns, label.phoneme,
            )
            .map_err(|e| anyhow::anyhow!("Failed to write to generated label file: {}", e))?;
        }
        let f0_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary f0 file: {}", e))?;
        let f0_path = f0_file.path().to_string_lossy().to_string();
        let mut buf_writer = std::io::BufWriter::new(&f0_file);
        for &f0 in f0_values {
            buf_writer.write_all(&f0.to_le_bytes()).map_err(|e| {
                anyhow::anyhow!("Failed to write f0 value to temporary f0 file: {}", e)
            })?;
        }
        buf_writer.flush().map_err(|e| {
            anyhow::anyhow!("Failed to flush temporary f0 file after writing: {}", e)
        })?;
        let melspec_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary melspec file: {}", e))?;
        let melspec_path = melspec_file.path().to_string_lossy().to_string();
        let generated_wav_file = tempfile::NamedTempFile::new()
            .map_err(|e| anyhow::anyhow!("Failed to create temporary wav file: {}", e))?;
        let generated_wav_path = generated_wav_file.path().to_string_lossy().to_string();
        let model_dir = self.neutrino_path.join("model").join(voice_id);
        let model_dir = model_dir.to_string_lossy().to_string();
        let cpu_count = num_cpus::get().to_string();
        self.invoke_client(
            &[
                label_path.as_str(),
                generated_label_path.as_str(),
                f0_path.as_str(),
                melspec_path.as_str(),
                generated_wav_path.as_str(),
                model_dir.as_str(),
                "-n",
                cpu_count.as_str(),
                "-m",
                "-t",
                "--skip-timing",
                "--skip-f0",
            ],
            cancel_token,
        )?;
        let (wav_header, samples) =
            wav_io::read_from_file(std::fs::File::open(&generated_wav_path)?)
                .map_err(|e| anyhow::anyhow!("Failed to parse generated wav data: {}", e))?;
        Ok((wav_header, samples))
    }

    fn invoke_client(
        &mut self,
        args: &[&str],
        cancel_token: &Arc<AtomicBool>,
    ) -> anyhow::Result<String> {
        self.spawn_server()?;
        let client_path = std::path::Path::new(&self.neutrino_path)
            .join("bin")
            .join("neutrino_client.exe");
        info!("Invoking Neutrino client with args: {}", args.join(" "));
        if !client_path.exists() {
            error!(
                "Neutrino client executable not found at: {}",
                client_path.display()
            );
            return Err(anyhow::anyhow!(
                "Neutrino client executable not found at: {}",
                client_path.display()
            ));
        }

        let mut child = std::process::Command::new(client_path)
            .args(args)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute Neutrino client: {}", e))?;

        let mut cancel_deadline = None;

        loop {
            if cancel_token.load(Ordering::SeqCst) {
                cancel_deadline.get_or_insert_with(|| {
                    info!("Cancellation detected while waiting for Neutrino client");
                    Instant::now() + CANCEL_GRACE_PERIOD
                });
            }

            if let Some(status) = child
                .try_wait()
                .map_err(|e| anyhow::anyhow!("Failed to wait for Neutrino client: {}", e))?
            {
                let output = child.wait_with_output().map_err(|e| {
                    anyhow::anyhow!("Failed to collect Neutrino client output: {}", e)
                })?;
                debug!(
                    "Neutrino client exited success={} stdout_bytes={} stderr_bytes={}",
                    status.success(),
                    output.stdout.len(),
                    output.stderr.len()
                );
                return Self::parse_client_output(status.success(), &output.stdout, &output.stderr);
            }

            if let Some(deadline) = cancel_deadline {
                if Instant::now() >= deadline {
                    error!(
                        "Neutrino client did not finish within {:?} after cancellation",
                        CANCEL_GRACE_PERIOD
                    );
                    if let Err(e) = child.kill() {
                        error!("Failed to kill Neutrino client process: {}", e);
                    } else {
                        info!("Killed Neutrino client process after cancellation timeout");
                    }
                    let _ = child.wait();
                    self.restart_server()?;
                    return Err(anyhow::anyhow!(
                        "Synthesis cancelled after timeout; Neutrino server restarted"
                    ));
                }
            }

            std::thread::sleep(CLIENT_POLL_INTERVAL);
        }
    }

    fn parse_client_output(success: bool, stdout: &[u8], stderr: &[u8]) -> anyhow::Result<String> {
        if success {
            let output = String::from_utf8_lossy(stdout).to_string();
            if output.contains("Error: ") || output.contains("Recv failed: ") {
                Err(anyhow::anyhow!("Neutrino client error: {}", output))
            } else {
                Ok(output)
            }
        } else {
            Err(anyhow::anyhow!(
                "Neutrino client error: {}",
                String::from_utf8_lossy(stderr)
            ))
        }
    }

    fn try_shutdown_server(&mut self) -> anyhow::Result<()> {
        let client_path = std::path::Path::new(&self.neutrino_path)
            .join("bin")
            .join("neutrino_client.exe");
        info!("Sending shutdown command to Neutrino server");
        if !client_path.exists() {
            error!(
                "Neutrino client executable not found at: {}",
                client_path.display()
            );
            return Err(anyhow::anyhow!(
                "Neutrino client executable not found at: {}",
                client_path.display()
            ));
        }

        let mut child = std::process::Command::new(client_path)
            .arg("shutdown")
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to execute Neutrino client shutdown: {}", e))?;
        let deadline = Instant::now() + SERVER_SHUTDOWN_TIMEOUT;

        loop {
            if let Some(status) = child.try_wait().map_err(|e| {
                anyhow::anyhow!("Failed to wait for Neutrino shutdown client: {}", e)
            })? {
                let output = child.wait_with_output().map_err(|e| {
                    anyhow::anyhow!("Failed to collect Neutrino shutdown output: {}", e)
                })?;
                let stdout =
                    Self::parse_client_output(status.success(), &output.stdout, &output.stderr)?;
                info!("Neutrino server shutdown response: {}", stdout);
                println!("Neutrino server shutdown response: {}", stdout);
                return Ok(());
            }

            if Instant::now() >= deadline {
                if let Err(e) = child.kill() {
                    error!("Failed to kill Neutrino shutdown client process: {}", e);
                } else {
                    info!("Killed Neutrino shutdown client after timeout");
                }
                let _ = child.wait();
                error!("Timed out waiting for Neutrino server shutdown");
                return Err(anyhow::anyhow!(
                    "Timed out waiting for Neutrino server shutdown"
                ));
            }

            std::thread::sleep(CLIENT_POLL_INTERVAL);
        }
    }

    fn force_kill_server(&mut self) {
        if let Some(server) = self.server.as_mut() {
            if let Err(e) = server.kill() {
                error!("Failed to kill Neutrino server process: {}", e);
            } else {
                info!("Neutrino server process killed successfully");
                println!("Neutrino server process killed successfully");
            }
            let _ = server.wait();
        }
        self.server = None;
    }

    fn stop_server(&mut self) {
        if self.server.is_none() {
            return;
        }

        if let Err(e) = self.try_shutdown_server() {
            error!("Failed to send shutdown command to Neutrino server: {}", e);
            self.force_kill_server();
            return;
        }

        if let Some(server) = self.server.as_mut() {
            let _ = server.wait();
        }
        self.server = None;
    }

    fn restart_server(&mut self) -> anyhow::Result<()> {
        info!("Restarting Neutrino server");
        self.stop_server();
        let result = self.spawn_server();
        if result.is_ok() {
            info!("Neutrino server restarted successfully");
        }
        result
    }

    pub fn shutdown(&mut self) {
        info!("Shutting down engine. dll_dir={}", self.dll_path.display());
        self.stop_server();
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn init_logger(dll_path: &Path) {
    let log_path = Engine::build_log_path(dll_path);
    LOGGER_INIT.call_once(|| {
        let mut builder =
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
        builder.format_timestamp_millis();

        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
        } else {
            builder.target(env_logger::Target::Pipe(Box::new(std::io::sink())));
        }

        let _ = builder.try_init();
    });
}
