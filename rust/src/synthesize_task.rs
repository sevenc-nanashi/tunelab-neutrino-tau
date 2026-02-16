#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisTaskPayload {
    start_time: f64,
    end_time: f64,
    duration: f64,
    part_properties: std::collections::HashMap<String, serde_json::Value>,
    notes: Vec<SynthesisNotePayload>,
    pitch: PitchPayload,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisNotePayload {
    start_time: f64,
    end_time: f64,
    pitch: i32,
    lyric: String,
    last_index: Option<usize>,
    next_index: Option<usize>,
    properties: std::collections::HashMap<String, serde_json::Value>,
    phonemes: Vec<SynthesisPhonemePayload>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisPhonemePayload {
    symbol: String,
    start_time: f64,
    end_time: f64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PitchPayload {
    times: Vec<f64>,
    values: Vec<LooseF64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SynthesisResponse {
    sample_rate: i32,
    sample_count: i32,
    samples: Vec<f32>,
    pitch_times: Vec<f64>,
    pitch_values: Vec<f64>,
    note_count: usize,
    phoneme_count: usize,
    property_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct LooseF64(f64);

impl LooseF64 {
    fn is_finite(self) -> bool {
        self.0.is_finite()
    }
}

impl<'de> serde::Deserialize<'de> for LooseF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(normalize_loose_f64)
                .map(LooseF64)
                .ok_or_else(|| serde::de::Error::custom("invalid float value")),
            serde_json::Value::String(s) => match s.as_str() {
                "NaN" => Ok(LooseF64(f64::NAN)),
                "Infinity" | "+Infinity" => Ok(LooseF64(f64::INFINITY)),
                "-Infinity" => Ok(LooseF64(f64::NEG_INFINITY)),
                _ => s
                    .parse::<f64>()
                    .map(normalize_loose_f64)
                    .map(LooseF64)
                    .map_err(|_| serde::de::Error::custom("invalid float string")),
            },
            serde_json::Value::Null => Ok(LooseF64(f64::NAN)),
            _ => Err(serde::de::Error::custom("invalid float type")),
        }
    }
}

fn normalize_loose_f64(value: f64) -> f64 {
    // C# side replaces non-finite pitch values with -double.MaxValue so JSON can be serialized.
    // Interpret that sentinel back as NaN on Rust side.
    if value == -f64::MAX {
        f64::NAN
    } else {
        value
    }
}

fn midi_note_to_frequency_hz(midi_note: i32) -> f32 {
    440.0_f32 * 2.0_f32.powf((midi_note as f32 - 69.0_f32) / 12.0_f32)
}

fn generate_note_based_sine_samples(
    notes: &[SynthesisNotePayload],
    sample_count: usize,
    sample_rate: i32,
    timeline_start_time: f64,
) -> Vec<f32> {
    if sample_count == 0 || sample_rate <= 0 {
        return Vec::new();
    }

    let sr = sample_rate as f32;
    let mut samples = vec![0.0_f32; sample_count];

    for note in notes {
        if !note.start_time.is_finite()
            || !note.end_time.is_finite()
            || note.end_time <= note.start_time
        {
            continue;
        }

        let start_idx =
            ((note.start_time - timeline_start_time) * sample_rate as f64).floor() as isize;
        let end_idx = ((note.end_time - timeline_start_time) * sample_rate as f64).ceil() as isize;

        let clamped_start = start_idx.max(0) as usize;
        let clamped_end = end_idx.min(sample_count as isize).max(0) as usize;
        if clamped_start >= clamped_end {
            continue;
        }

        let frequency = midi_note_to_frequency_hz(note.pitch);
        if !frequency.is_finite() || frequency <= 0.0 {
            continue;
        }

        let note_len = clamped_end - clamped_start;
        let fade_len = note_len.min(64) / 2;
        let amplitude = 0.12_f32;

        for i in clamped_start..clamped_end {
            let absolute_time = timeline_start_time as f32 + i as f32 / sr;
            let note_time = absolute_time - note.start_time as f32;
            let mut gain = amplitude;

            if fade_len > 0 {
                let pos = i - clamped_start;
                let tail = clamped_end - i - 1;
                if pos < fade_len {
                    gain *= pos as f32 / fade_len as f32;
                } else if tail < fade_len {
                    gain *= tail as f32 / fade_len as f32;
                }
            }

            samples[i] += (std::f32::consts::TAU * frequency * note_time).sin() * gain;
        }
    }

    for sample in &mut samples {
        *sample = sample.clamp(-1.0, 1.0);
    }

    samples
}

fn clamp_sample_count(duration_sec: f64, sample_rate: i32) -> i32 {
    if !duration_sec.is_finite() || duration_sec <= 0.0 {
        return 0;
    }
    if sample_rate <= 0 {
        return 0;
    }

    let count = (duration_sec * sample_rate as f64).round();
    if !count.is_finite() || count <= 0.0 {
        return 0;
    }

    if count >= i32::MAX as f64 {
        i32::MAX
    } else {
        count as i32
    }
}

pub fn default_sample_rate() -> i32 {
    44_100
}

pub fn synthesize_task_json(payload_json: &str) -> Result<String, String> {
    let payload: SynthesisTaskPayload = serde_json::from_str(payload_json)
        .map_err(|e| format!("Failed to parse synthesis task payload: {e}"))?;

    let sample_rate = default_sample_rate();
    let duration = if payload.duration.is_finite() {
        payload.duration
    } else {
        payload.end_time - payload.start_time
    };
    let sample_count = clamp_sample_count(duration.max(0.0), sample_rate);
    let phoneme_count = payload.notes.iter().map(|note| note.phonemes.len()).sum();
    let property_count = payload.part_properties.len()
        + payload
            .notes
            .iter()
            .map(|note| note.properties.len())
            .sum::<usize>();

    let samples = generate_note_based_sine_samples(
        &payload.notes,
        sample_count.max(0) as usize,
        sample_rate,
        payload.start_time,
    );
    let (pitch_times, pitch_values): (Vec<f64>, Vec<f64>) = payload
        .pitch
        .times
        .iter()
        .zip(payload.pitch.values.iter())
        .filter_map(|(time, value)| {
            let y = value.0;
            if time.is_finite() && y.is_finite() {
                Some((*time, y))
            } else {
                None
            }
        })
        .unzip();

    let response = SynthesisResponse {
        sample_rate,
        sample_count,
        samples,
        pitch_times,
        pitch_values,
        note_count: payload.notes.len(),
        phoneme_count,
        property_count,
    };

    serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize synthesis response: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_accepts_nan_like_pitch_values() {
        let payload = r#"{
            "startTime": 0.0,
            "endTime": 1.0,
            "duration": 1.0,
            "partProperties": {},
            "notes": [],
            "pitch": {
                "times": [0.0, 0.5, 1.0],
                "values": ["NaN", null, 440.0]
            }
        }"#;

        let result = synthesize_task_json(payload);
        assert!(
            result.is_ok(),
            "payload with NaN-like values must be accepted"
        );
    }

    #[test]
    fn loose_f64_treats_negative_max_as_nan() {
        let v: LooseF64 = serde_json::from_str("-1.7976931348623157e308").expect("must parse");
        assert!(!v.is_finite());
    }

    #[test]
    fn note_based_sine_samples_are_generated() {
        let notes = vec![SynthesisNotePayload {
            start_time: 0.0,
            end_time: 0.5,
            pitch: 69,
            lyric: "a".into(),
            last_index: None,
            next_index: None,
            properties: std::collections::HashMap::new(),
            phonemes: vec![],
        }];
        let samples = generate_note_based_sine_samples(&notes, 128, 44_100, 0.0);
        assert_eq!(samples.len(), 128);
        assert!(samples.iter().any(|x| x.abs() > 0.0));
    }
}
