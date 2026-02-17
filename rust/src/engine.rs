use crate::config;
use itertools::Itertools;
use std::io::Write;
use std::os::windows::process::CommandExt;

#[derive(Debug)]
pub struct Engine {
    neutrino_path: std::path::PathBuf,
    server: Option<std::process::Child>,
}

type WavData = (wav_io::header::WavHeader, Vec<f32>);

impl Engine {
    pub fn new(dll_path: std::path::PathBuf) -> anyhow::Result<Self> {
        let config_path = dll_path.join("config.json");
        let mut config = if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
            serde_json::from_str(&config_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?
        } else {
            config::Config::default()
        };
        if config.neutrino_path.is_none() {
            if let Some(result) = native_dialog::FileDialogBuilder::default()
                .set_title("Select neutrino.exe")
                .add_filter("Executable", ["exe"])
                .open_single_file()
                .show()?
            {
                if !result.exists() {
                    return Err(anyhow::anyhow!(
                        "Selected Neutrino path does not exist: {}",
                        result.display()
                    ));
                }
                if result.file_name().and_then(|n| n.to_str()) != Some("neutrino.exe") {
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

                config.neutrino_path = Some(neutrino_root.to_string_lossy().to_string());

                std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
                    .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;
            } else {
                return Err(anyhow::anyhow!(
                    "Neutrino path is required but not provided"
                ));
            }
        }

        let neutrino_path = config.neutrino_path.as_ref().unwrap();
        if !std::path::Path::new(neutrino_path).exists() {
            return Err(anyhow::anyhow!(
                "Neutrino path does not exist: {}",
                neutrino_path
            ));
        }

        Ok(Self {
            neutrino_path: config.neutrino_path.unwrap().into(),
            server: None,
        })
    }

    fn spawn_server(&mut self) -> anyhow::Result<()> {
        if self
            .server
            .as_mut()
            .is_some_and(|s| s.try_wait().map(|w| w.is_none()).unwrap_or(false))
        {
            return Ok(());
        }
        let server_path = self.neutrino_path.join("bin").join("neutrino_server.exe");
        if !server_path.exists() {
            return Err(anyhow::anyhow!(
                "Neutrino server executable not found at: {}",
                server_path.display()
            ));
        }

        let child = std::process::Command::new(server_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn Neutrino server: {}", e))?;

        self.server = Some(child);
        Ok(())
    }

    pub fn load_voices(&self) -> anyhow::Result<Vec<crate::speaker::VoiceSource>> {
        let mut speakers = Vec::new();
        let models_path = self.neutrino_path.join("model");
        if !models_path.exists() {
            return Err(anyhow::anyhow!("Neutrino model directory not found"));
        }

        for entry in std::fs::read_dir(models_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                match crate::speaker::VoiceSource::load(&entry.path()) {
                    Ok(voice) => speakers.push(voice),
                    Err(e) => eprintln!(
                        "Failed to load voice from {}: {}",
                        entry.path().display(),
                        e
                    ),
                }
            }
        }

        Ok(speakers)
    }

    pub fn synthesize(&mut self, synthesis_task_json: &str) -> anyhow::Result<String> {
        let (payload, score, tunelab_start_in_synthesis_time) =
            Self::prepare_synthesis_input(synthesis_task_json)?;

        let timings = self.synthesize_timing(&payload.voice_id, &score)?;
        let mapped_phoneme_groups = self.map_phonemes_to_notes(&score, &timings)?;
        let merged_phonemes = Self::merge_phonemes_with_payload(
            &payload,
            &mapped_phoneme_groups,
            tunelab_start_in_synthesis_time,
        );

        let style_score = Self::transpose_score_pitches(&score, payload.style_shift);
        let inferred_f0_values =
            self.synthesize_f0(&payload.voice_id, &style_score, &merged_phonemes)?;
        // Infer f0 on style-shifted notes, then shift f0 back to the original key.
        let f0_values = Self::shift_f0_by_semitones(&inferred_f0_values, -payload.style_shift);

        let mapped_f0_values = Self::apply_payload_pitch_to_f0(
            &payload.pitch,
            &f0_values,
            tunelab_start_in_synthesis_time,
        );

        let waveform_score =
            Self::transpose_score_pitches(&style_score, payload.waveform_style_shift);
        let wav_data = self.synthesize_waveform(
            &payload.voice_id,
            &waveform_score,
            &merged_phonemes,
            &mapped_f0_values,
        )?;
        let response = Self::build_synthesis_response(
            &payload,
            &f0_values,
            &mapped_phoneme_groups,
            &merged_phonemes,
            wav_data,
            tunelab_start_in_synthesis_time,
        );

        Ok(serde_json::to_string(&response)?)
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

        crate::synthesizer::SynthesisResponse {
            start_time: -tunelab_start_in_synthesis_time,
            sample_rate: wav_data.0.sample_rate as _,
            sample_count: wav_data.1.len() as _,
            samples: wav_data.1,
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
        self.invoke_client(&[
            &label_path,
            generated_label_path.as_str(),
            f0_path.as_str(),
            melspec_path.as_str(),
            generated_wav_path.as_str(),
            self.neutrino_path
                .join("model")
                .join(voice_id)
                .to_str()
                .unwrap(),
            "-n",
            num_cpus::get().to_string().as_str(),
            "-m",
            "-t",
            "--skip-melspec",
            "--skip-f0",
            "--skip-wav",
        ])?;
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
        self.invoke_client(&[
            &label_path,
            generated_label_path.as_str(),
            f0_path.as_str(),
            melspec_path.as_str(),
            generated_wav_path.as_str(),
            self.neutrino_path
                .join("model")
                .join(voice_id)
                .to_str()
                .unwrap(),
            "-n",
            num_cpus::get().to_string().as_str(),
            "-m",
            "-t",
            "--skip-timing",
            "--skip-melspec",
            "--skip-wav",
        ])?;
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
        self.invoke_client(&[
            &label_path,
            generated_label_path.as_str(),
            f0_path.as_str(),
            melspec_path.as_str(),
            generated_wav_path.as_str(),
            self.neutrino_path
                .join("model")
                .join(voice_id)
                .to_str()
                .unwrap(),
            "-n",
            num_cpus::get().to_string().as_str(),
            "-m",
            "-t",
            "--skip-timing",
            "--skip-f0",
        ])?;
        let (wav_header, samples) =
            wav_io::read_from_file(std::fs::File::open(&generated_wav_path)?)
                .map_err(|e| anyhow::anyhow!("Failed to parse generated wav data: {}", e))?;
        Ok((wav_header, samples))
    }

    fn invoke_client(&mut self, args: &[&str]) -> anyhow::Result<String> {
        self.spawn_server()?;
        let client_path = std::path::Path::new(&self.neutrino_path)
            .join("bin")
            .join("neutrino_client.exe");
        if !client_path.exists() {
            return Err(anyhow::anyhow!(
                "Neutrino client executable not found at: {}",
                client_path.display()
            ));
        }

        let output = std::process::Command::new(client_path)
            .args(args)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute Neutrino client: {}", e))?;

        if output.status.success() {
            let output = String::from_utf8_lossy(&output.stdout).to_string();
            if output.contains("Error: ") || output.contains("Recv failed: ") {
                Err(anyhow::anyhow!("Neutrino client error: {}", output))
            } else {
                Ok(output)
            }
        } else {
            Err(anyhow::anyhow!(
                "Neutrino client error: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub fn shutdown(&mut self) {
        if let Ok(status) = self.invoke_client(&["shutdown"]) {
            println!("Neutrino server shutdown response: {}", status);
        } else {
            eprintln!("Failed to send shutdown command to Neutrino server");

            if let Err(e) = self.server.as_mut().unwrap().kill() {
                eprintln!("Failed to kill Neutrino server process: {}", e);
            } else {
                println!("Neutrino server process killed successfully");
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}
