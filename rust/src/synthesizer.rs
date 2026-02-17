use wana_kana::ConvertJapanese;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynthesisTaskPayload {
    pub voice_id: String,
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
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
    pub sample_rate: i32,
    pub sample_count: i32,
    pub samples: Vec<f32>,
    pub pitch_times: Vec<f64>,
    pub pitch_values: Vec<f64>,
    pub note_count: usize,
    pub phoneme_count: usize,
    pub property_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct LooseF64(pub f64);

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

pub fn mora_to_phonemes(mora: &str) -> anyhow::Result<Vec<&'static str>> {
    Ok(match mora.to_katakana().as_str() {
        "ア" => vec!["a"],
        "イ" => vec!["i"],
        "ウ" => vec!["u"],
        "エ" => vec!["e"],
        "オ" => vec!["o"],
        "キャ" => vec!["ky", "a"],
        "キュ" => vec!["ky", "u"],
        "キェ" => vec!["ky", "e"],
        "キョ" => vec!["ky", "o"],
        "カ" => vec!["k", "a"],
        "キ" => vec!["k", "i"],
        "ク" => vec!["k", "u"],
        "ケ" => vec!["k", "e"],
        "コ" => vec!["k", "o"],
        "シャ" => vec!["sh", "a"],
        "スィ" => vec!["s", "i"],
        "シュ" => vec!["sh", "u"],
        "シェ" => vec!["sh", "e"],
        "ショ" => vec!["sh", "o"],
        "サ" => vec!["s", "a"],
        "シ" => vec!["sh", "i"],
        "ス" => vec!["s", "u"],
        "セ" => vec!["s", "e"],
        "ソ" => vec!["s", "o"],
        "チャ" => vec!["ch", "a"],
        "チュ" => vec!["ch", "u"],
        "チェ" => vec!["ch", "e"],
        "チョ" => vec!["ch", "o"],
        "タ" => vec!["t", "a"],
        "チ" => vec!["ch", "i"],
        "ツ" => vec!["ts", "u"],
        "テ" => vec!["t", "e"],
        "ト" => vec!["t", "o"],
        "ツァ" => vec!["ts", "a"],
        "ツィ" => vec!["ts", "i"],
        "ツェ" => vec!["ts", "e"],
        "ツォ" => vec!["ts", "o"],
        "ナ" => vec!["n", "a"],
        "ニ" => vec!["n", "i"],
        "ヌ" => vec!["n", "u"],
        "ネ" => vec!["n", "e"],
        "ノ" => vec!["n", "o"],
        "ニャ" => vec!["ny", "a"],
        "ニュ" => vec!["ny", "u"],
        "ニェ" => vec!["ny", "e"],
        "ニョ" => vec!["ny", "o"],
        "ハ" => vec!["h", "a"],
        "ヒ" => vec!["h", "i"],
        "フ" => vec!["h", "u"],
        "ヘ" => vec!["h", "e"],
        "ホ" => vec!["h", "o"],
        "ヒャ" => vec!["hy", "a"],
        "ヒュ" => vec!["hy", "u"],
        "ヒェ" => vec!["hy", "e"],
        "ヒョ" => vec!["hy", "o"],
        "マ" => vec!["m", "a"],
        "ミ" => vec!["m", "i"],
        "ム" => vec!["m", "u"],
        "メ" => vec!["m", "e"],
        "モ" => vec!["m", "o"],
        "ファ" => vec!["f", "a"],
        "フィ" => vec!["f", "i"],
        "フェ" => vec!["f", "e"],
        "フォ" => vec!["f", "o"],
        "ヤ" => vec!["y", "a"],
        "ユ" => vec!["y", "u"],
        "イェ" => vec!["y", "e"],
        "ヨ" => vec!["y", "o"],
        "ミャ" => vec!["my", "a"],
        "ミュ" => vec!["my", "u"],
        "ミェ" => vec!["my", "e"],
        "ミョ" => vec!["my", "o"],
        "ラ" => vec!["r", "a"],
        "リ" => vec!["r", "i"],
        "ル" => vec!["r", "u"],
        "レ" => vec!["r", "e"],
        "ロ" => vec!["r", "o"],
        "リャ" => vec!["ry", "a"],
        "リュ" => vec!["ry", "u"],
        "リェ" => vec!["ry", "e"],
        "リョ" => vec!["ry", "o"],
        "ワ" => vec!["w", "a"],
        "ヲ" => vec!["o"],
        "ギャ" => vec!["gy", "a"],
        "ギュ" => vec!["gy", "u"],
        "ギェ" => vec!["gy", "e"],
        "ギョ" => vec!["gy", "o"],
        "ジャ" => vec!["j", "a"],
        "ジュ" => vec!["j", "u"],
        "ジェ" => vec!["j", "e"],
        "ジョ" => vec!["j", "o"],
        "ガ" => vec!["g", "a"],
        "ギ" => vec!["g", "i"],
        "グ" => vec!["g", "u"],
        "ゲ" => vec!["g", "e"],
        "ゴ" => vec!["g", "o"],
        "ビャ" => vec!["by", "a"],
        "ビュ" => vec!["by", "u"],
        "ビェ" => vec!["by", "e"],
        "ビョ" => vec!["by", "o"],
        "ザ" => vec!["z", "a"],
        "ジ" => vec!["j", "i"],
        "ズ" => vec!["z", "u"],
        "ゼ" => vec!["z", "e"],
        "ゾ" => vec!["z", "o"],
        "ピャ" => vec!["py", "a"],
        "ピュ" => vec!["py", "u"],
        "ピェ" => vec!["py", "e"],
        "ピョ" => vec!["py", "o"],
        "ダ" => vec!["d", "a"],
        "ヂ" => vec!["j", "i"],
        "ヅ" => vec!["z", "u"],
        "デ" => vec!["d", "e"],
        "ド" => vec!["d", "o"],
        "ヴァ" => vec!["v", "a"],
        "ヴィ" => vec!["v", "i"],
        "ヴ" => vec!["v", "u"],
        "ヴェ" => vec!["v", "e"],
        "ヴォ" => vec!["v", "o"],
        "バ" => vec!["b", "a"],
        "ビ" => vec!["b", "i"],
        "ブ" => vec!["b", "u"],
        "ベ" => vec!["b", "e"],
        "ボ" => vec!["b", "o"],
        "ウィ" => vec!["w", "i"],
        "ウェ" => vec!["w", "e"],
        "ウォ" => vec!["w", "o"],
        "パ" => vec!["p", "a"],
        "ピ" => vec!["p", "i"],
        "プ" => vec!["p", "u"],
        "ペ" => vec!["p", "e"],
        "ポ" => vec!["p", "o"],
        "ディ" => vec!["d", "i"],
        "デュ" => vec!["dy", "u"],
        "トゥ" => vec!["t", "u"],
        "ドゥ" => vec!["d", "u"],
        "ン" => vec!["N"],
        "ッ" => vec!["cl"],
        "ズィ" => vec!["z", "i"],
        _ => anyhow::bail!("Unsupported mora: {}", mora),
    })
}

pub fn task_notes_to_score(
    notes: &[SynthesisNotePayload],
) -> anyhow::Result<crate::neutrino_score::Score> {
    // Synthesis task times are in seconds.
    // Until tempo is provided by payload, use the default score tempo.
    let bpm = 120.0;
    let mut score = crate::neutrino_score::Score {
        tempo: bpm,
        ..Default::default()
    };

    score.notes.push(crate::neutrino_score::Note {
        pitch: 60,
        start_time_ns: 0,
        length: crate::neutrino_score::NoteLength::from_4th_note(1),
        language: Some("JPN".to_string()),
        language_dependent_context: Some("p".to_string()),
        phonemes: vec!["pau".to_string()],
    });
    for note in notes {
        let phonemes: Vec<String> = if note.phonemes.is_empty() {
            mora_to_phonemes(&note.lyric)?
                .into_iter()
                .map(ToString::to_string)
                .collect()
        } else {
            note.phonemes.iter().map(|p| p.symbol.clone()).collect()
        };

        let start_time_ns = if note.start_time.is_finite() && note.start_time > 0.0 {
            (note.start_time * 1_000_000_000.0).round() as u64
        } else {
            0
        };

        score.notes.push(crate::neutrino_score::Note {
            pitch: note.pitch.clamp(0, 127) as u8,
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
        pitch: 60,
        start_time_ns: score
            .notes
            .last()
            .map(|n| n.start_time_ns + n.length.to_nanoseconds(bpm))
            .unwrap_or(0),
        length: crate::neutrino_score::NoteLength::from_4th_note(1),
        phonemes: vec!["pau".to_string()],
        language: Some("JPN".to_string()),
        language_dependent_context: Some("p".to_string()),
    });

    Ok(score)
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
