use wana_kana::ConvertJapanese;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisTaskPayload {
    pub voice_id: String,
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    #[serde(default)]
    pub style_shift: f64,
    pub part_properties: std::collections::HashMap<String, serde_json::Value>,
    pub notes: Vec<SynthesisNotePayload>,
    pub pitch: PitchPayload,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisNotePayload {
    pub start_time: f64,
    pub end_time: f64,
    pub pitch: i32,
    pub lyric: String,
    pub last_index: Option<usize>,
    pub next_index: Option<usize>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub phonemes: Vec<SynthesisPhonemePayload>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisPhonemePayload {
    pub symbol: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchPayload {
    pub times: Vec<f64>,
    pub values: Vec<LooseF64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisResponse {
    pub start_time: f64,
    pub sample_rate: i32,
    pub sample_count: i32,
    pub samples: Vec<f32>,
    pub pitch_times: Vec<f64>,
    pub pitch_values: Vec<f64>,
    pub note_phonemes: Vec<NotePhonemes>,
    pub note_count: usize,
    pub phoneme_count: usize,
    pub property_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesizedPhoneme {
    pub symbol: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotePhonemes {
    pub note_index: usize,
    pub phonemes: Vec<SynthesizedPhoneme>,
}

#[derive(Debug, Clone, Copy)]
pub struct LooseF64(pub f64);

impl LooseF64 {
    pub fn is_finite(self) -> bool {
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
impl serde::Serialize for LooseF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.0.is_nan() {
            serializer.serialize_f64(-f64::MAX)
        } else if self.0.is_infinite() {
            serializer.serialize_f64(if self.0.is_sign_positive() {
                f64::MAX
            } else {
                -f64::MAX
            })
        } else {
            serializer.serialize_f64(self.0)
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

static PHONEMES: &[&str] = &[
    "a", "i", "u", "e", "o", "ky", "k", "sh", "s", "ch", "t", "ts", "n", "ny", "h", "hy", "m", "f",
    "y", "my", "r", "ry", "w", "gy", "j", "g", "by", "z", "py", "d", "v", "b", "p", "dy", "N",
    "cl", "br", "pau", "sil",
];
pub fn mora_to_phonemes(mora: &str) -> anyhow::Result<Vec<String>> {
    Ok(match mora.to_katakana().as_str() {
        "ア" => vec!["a".to_string()],
        "イ" => vec!["i".to_string()],
        "ウ" => vec!["u".to_string()],
        "エ" => vec!["e".to_string()],
        "オ" => vec!["o".to_string()],
        "キャ" => vec!["ky".to_string(), "a".to_string()],
        "キュ" => vec!["ky".to_string(), "u".to_string()],
        "キェ" => vec!["ky".to_string(), "e".to_string()],
        "キョ" => vec!["ky".to_string(), "o".to_string()],
        "カ" => vec!["k".to_string(), "a".to_string()],
        "キ" => vec!["k".to_string(), "i".to_string()],
        "ク" => vec!["k".to_string(), "u".to_string()],
        "ケ" => vec!["k".to_string(), "e".to_string()],
        "コ" => vec!["k".to_string(), "o".to_string()],
        "シャ" => vec!["sh".to_string(), "a".to_string()],
        "スィ" => vec!["s".to_string(), "i".to_string()],
        "シュ" => vec!["sh".to_string(), "u".to_string()],
        "シェ" => vec!["sh".to_string(), "e".to_string()],
        "ショ" => vec!["sh".to_string(), "o".to_string()],
        "サ" => vec!["s".to_string(), "a".to_string()],
        "シ" => vec!["sh".to_string(), "i".to_string()],
        "ス" => vec!["s".to_string(), "u".to_string()],
        "セ" => vec!["s".to_string(), "e".to_string()],
        "ソ" => vec!["s".to_string(), "o".to_string()],
        "チャ" => vec!["ch".to_string(), "a".to_string()],
        "チュ" => vec!["ch".to_string(), "u".to_string()],
        "チェ" => vec!["ch".to_string(), "e".to_string()],
        "チョ" => vec!["ch".to_string(), "o".to_string()],
        "タ" => vec!["t".to_string(), "a".to_string()],
        "チ" => vec!["ch".to_string(), "i".to_string()],
        "ツ" => vec!["ts".to_string(), "u".to_string()],
        "テ" => vec!["t".to_string(), "e".to_string()],
        "ト" => vec!["t".to_string(), "o".to_string()],
        "ツァ" => vec!["ts".to_string(), "a".to_string()],
        "ツィ" => vec!["ts".to_string(), "i".to_string()],
        "ツェ" => vec!["ts".to_string(), "e".to_string()],
        "ツォ" => vec!["ts".to_string(), "o".to_string()],
        "ナ" => vec!["n".to_string(), "a".to_string()],
        "ニ" => vec!["n".to_string(), "i".to_string()],
        "ヌ" => vec!["n".to_string(), "u".to_string()],
        "ネ" => vec!["n".to_string(), "e".to_string()],
        "ノ" => vec!["n".to_string(), "o".to_string()],
        "ニャ" => vec!["ny".to_string(), "a".to_string()],
        "ニュ" => vec!["ny".to_string(), "u".to_string()],
        "ニェ" => vec!["ny".to_string(), "e".to_string()],
        "ニョ" => vec!["ny".to_string(), "o".to_string()],
        "ハ" => vec!["h".to_string(), "a".to_string()],
        "ヒ" => vec!["h".to_string(), "i".to_string()],
        "フ" => vec!["h".to_string(), "u".to_string()],
        "ヘ" => vec!["h".to_string(), "e".to_string()],
        "ホ" => vec!["h".to_string(), "o".to_string()],
        "ヒャ" => vec!["hy".to_string(), "a".to_string()],
        "ヒュ" => vec!["hy".to_string(), "u".to_string()],
        "ヒェ" => vec!["hy".to_string(), "e".to_string()],
        "ヒョ" => vec!["hy".to_string(), "o".to_string()],
        "マ" => vec!["m".to_string(), "a".to_string()],
        "ミ" => vec!["m".to_string(), "i".to_string()],
        "ム" => vec!["m".to_string(), "u".to_string()],
        "メ" => vec!["m".to_string(), "e".to_string()],
        "モ" => vec!["m".to_string(), "o".to_string()],
        "ファ" => vec!["f".to_string(), "a".to_string()],
        "フィ" => vec!["f".to_string(), "i".to_string()],
        "フェ" => vec!["f".to_string(), "e".to_string()],
        "フォ" => vec!["f".to_string(), "o".to_string()],
        "ヤ" => vec!["y".to_string(), "a".to_string()],
        "ユ" => vec!["y".to_string(), "u".to_string()],
        "イェ" => vec!["y".to_string(), "e".to_string()],
        "ヨ" => vec!["y".to_string(), "o".to_string()],
        "ミャ" => vec!["my".to_string(), "a".to_string()],
        "ミュ" => vec!["my".to_string(), "u".to_string()],
        "ミェ" => vec!["my".to_string(), "e".to_string()],
        "ミョ" => vec!["my".to_string(), "o".to_string()],
        "ラ" => vec!["r".to_string(), "a".to_string()],
        "リ" => vec!["r".to_string(), "i".to_string()],
        "ル" => vec!["r".to_string(), "u".to_string()],
        "レ" => vec!["r".to_string(), "e".to_string()],
        "ロ" => vec!["r".to_string(), "o".to_string()],
        "リャ" => vec!["ry".to_string(), "a".to_string()],
        "リュ" => vec!["ry".to_string(), "u".to_string()],
        "リェ" => vec!["ry".to_string(), "e".to_string()],
        "リョ" => vec!["ry".to_string(), "o".to_string()],
        "ワ" => vec!["w".to_string(), "a".to_string()],
        "ヲ" => vec!["o".to_string()],
        "ギャ" => vec!["gy".to_string(), "a".to_string()],
        "ギュ" => vec!["gy".to_string(), "u".to_string()],
        "ギェ" => vec!["gy".to_string(), "e".to_string()],
        "ギョ" => vec!["gy".to_string(), "o".to_string()],
        "ジャ" => vec!["j".to_string(), "a".to_string()],
        "ジュ" => vec!["j".to_string(), "u".to_string()],
        "ジェ" => vec!["j".to_string(), "e".to_string()],
        "ジョ" => vec!["j".to_string(), "o".to_string()],
        "ガ" => vec!["g".to_string(), "a".to_string()],
        "ギ" => vec!["g".to_string(), "i".to_string()],
        "グ" => vec!["g".to_string(), "u".to_string()],
        "ゲ" => vec!["g".to_string(), "e".to_string()],
        "ゴ" => vec!["g".to_string(), "o".to_string()],
        "ビャ" => vec!["by".to_string(), "a".to_string()],
        "ビュ" => vec!["by".to_string(), "u".to_string()],
        "ビェ" => vec!["by".to_string(), "e".to_string()],
        "ビョ" => vec!["by".to_string(), "o".to_string()],
        "ザ" => vec!["z".to_string(), "a".to_string()],
        "ジ" => vec!["j".to_string(), "i".to_string()],
        "ズ" => vec!["z".to_string(), "u".to_string()],
        "ゼ" => vec!["z".to_string(), "e".to_string()],
        "ゾ" => vec!["z".to_string(), "o".to_string()],
        "ピャ" => vec!["py".to_string(), "a".to_string()],
        "ピュ" => vec!["py".to_string(), "u".to_string()],
        "ピェ" => vec!["py".to_string(), "e".to_string()],
        "ピョ" => vec!["py".to_string(), "o".to_string()],
        "ダ" => vec!["d".to_string(), "a".to_string()],
        "ヂ" => vec!["j".to_string(), "i".to_string()],
        "ヅ" => vec!["z".to_string(), "u".to_string()],
        "デ" => vec!["d".to_string(), "e".to_string()],
        "ド" => vec!["d".to_string(), "o".to_string()],
        "ヴぁ" => vec!["v".to_string(), "a".to_string()],
        "ヴぃ" => vec!["v".to_string(), "i".to_string()],
        "ヴ" => vec!["v".to_string(), "u".to_string()],
        "ヴぇ" => vec!["v".to_string(), "e".to_string()],
        "ヴぉ" => vec!["v".to_string(), "o".to_string()],
        "バ" => vec!["b".to_string(), "a".to_string()],
        "ビ" => vec!["b".to_string(), "i".to_string()],
        "ブ" => vec!["b".to_string(), "u".to_string()],
        "ベ" => vec!["b".to_string(), "e".to_string()],
        "ボ" => vec!["b".to_string(), "o".to_string()],
        "ウィ" => vec!["w".to_string(), "i".to_string()],
        "ウェ" => vec!["w".to_string(), "e".to_string()],
        "ウォ" => vec!["w".to_string(), "o".to_string()],
        "パ" => vec!["p".to_string(), "a".to_string()],
        "ピ" => vec!["p".to_string(), "i".to_string()],
        "プ" => vec!["p".to_string(), "u".to_string()],
        "ペ" => vec!["p".to_string(), "e".to_string()],
        "ポ" => vec!["p".to_string(), "o".to_string()],
        "ディ" => vec!["d".to_string(), "i".to_string()],
        "デュ" => vec!["dy".to_string(), "u".to_string()],
        "トゥ" => vec!["t".to_string(), "u".to_string()],
        "ドゥ" => vec!["d".to_string(), "u".to_string()],
        "ン" => vec!["N".to_string()],
        "ッ" => vec!["cl".to_string()],
        "ズィ" => vec!["z".to_string(), "i".to_string()],
        _ if PHONEMES.contains(&mora.to_katakana().as_str()) => {
            vec![mora.to_katakana().to_lowercase()]
        }
        _ => anyhow::bail!("Unsupported mora: {}", mora),
    })
}

pub fn task_notes_to_score(
    notes: &[SynthesisNotePayload],
) -> anyhow::Result<crate::neutrino_score::Score> {
    if notes.is_empty() {
        anyhow::bail!("No notes provided in synthesis task payload");
    }
    // Synthesis task times are in seconds.
    // Until tempo is provided by payload, use `bpm = 60000` to interpret note lengths as
    // milliseconds.
    let bpm = 60000.0;
    let mut score = crate::neutrino_score::Score {
        tempo: bpm,
        ..Default::default()
    };

    let first_pau = crate::neutrino_score::Note {
        pitch: None,
        start_time_ns: 0,
        length: crate::neutrino_score::NoteLength::from_seconds_float(1.0, bpm),
        language: Some("JPN".to_string()),
        language_dependent_context: Some("p".to_string()),
        phonemes: vec!["pau".to_string()],
    };
    let first_pau_length_ns = first_pau.length.to_nanoseconds(bpm);
    score.notes.push(first_pau);
    let first_note_start_time = notes[0].start_time;
    for note in notes {
        let phonemes: Vec<String> = if note.phonemes.is_empty() {
            mora_to_phonemes(&note.lyric)?
        } else {
            note.phonemes.iter().map(|p| p.symbol.clone()).collect()
        };

        let start_time_ns = ((note.start_time - first_note_start_time).max(0.0) * 1_000_000_000.0)
            .round() as u64
            + first_pau_length_ns;

        score.notes.push(crate::neutrino_score::Note {
            pitch: Some(note.pitch.clamp(0, 127) as u8),
            start_time_ns,
            length: crate::neutrino_score::NoteLength::from_4th_note_float(
                (note.end_time - note.start_time).max(0.0) * (bpm / 60.0),
            ),
            phonemes,
            language: Some("JPN".to_string()),
            language_dependent_context: Some("0".to_string()),
        });
    }
    score.notes.push(crate::neutrino_score::Note {
        pitch: None,
        start_time_ns: score
            .notes
            .last()
            .map(|n| n.start_time_ns + n.length.to_nanoseconds(bpm))
            .unwrap_or(0),
        length: crate::neutrino_score::NoteLength::from_seconds_float(1.0, bpm),
        phonemes: vec!["pau".to_string()],
        language: Some("JPN".to_string()),
        language_dependent_context: Some("p".to_string()),
    });

    Ok(score)
}

#[derive(Debug, Clone)]
pub struct TimingLabel {
    pub start_time_ns: u64,
    pub end_time_ns: u64,
    pub phoneme: String,
}

pub fn parse_timing_label_file(label_file_content: &str) -> anyhow::Result<Vec<TimingLabel>> {
    let mut labels = Vec::new();
    for line in label_file_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let start_time_100ns = parts[0]
            .parse::<u64>()
            .map_err(|e| anyhow::anyhow!("Failed to parse start time in label file: {}", e))?;
        let end_time_100ns = parts[1]
            .parse::<u64>()
            .map_err(|e| anyhow::anyhow!("Failed to parse end time in label file: {}", e))?;
        let phoneme = parts[2].to_string();
        labels.push(TimingLabel {
            start_time_ns: start_time_100ns * 100,
            end_time_ns: end_time_100ns * 100,
            phoneme,
        });
    }

    Ok(labels)
}

pub fn freq_to_midi(freq: f32) -> f32 {
    69.0 + 12.0 * (freq / 440.0).log2()
}
pub fn midi_to_freq(midi: f32) -> f32 {
    440.0 * 2.0_f32.powf((midi - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loose_f64_treats_negative_max_as_nan() {
        let v: LooseF64 = serde_json::from_str("-1.7976931348623157e308").expect("must parse");
        assert!(!v.is_finite());
    }
}
