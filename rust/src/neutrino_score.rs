use crate::neutrino_label::{parse_label_line, Label, LabelValue};

const TEMPLATE_LABEL_LINE: &str = "p@xx^xx-pau+r=a_xx%xx^00_00~00-1!1[xx$xx]xx/A:xx-xx-xx@xx~xx/B:1_1_1@xx|xx/C:2+1+1@JPN&0/D:xx!xx#xx$xx%xx|xx&xx;xx-xx/E:xx]xx^0=4/4~100!1@240#96+xx]1$1|0[24&0]96=0^100~xx#xx_xx;xx$xx&xx%xx[xx|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~xx+xx!xx^xx/F:C5#0#0-4/4$100$1+60%24;xx/G:xx_xx/H:xx_xx/I:8_8/J:2~2@1";
const XX: &str = "xx";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteLength(i32);

impl From<NoteLength> for i32 {
    fn from(length: NoteLength) -> Self {
        length.0
    }
}
impl std::fmt::Display for NoteLength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl NoteLength {
    pub fn from_4th_note(count: i32) -> Self {
        Self(count * 24)
    }
    pub fn from_4th_note_float(count: f64) -> Self {
        Self((count * 24.0).round() as i32)
    }
    pub fn from_8th_note(count: i32) -> Self {
        Self(count * 12)
    }
    pub fn from_8th_note_float(count: f64) -> Self {
        Self((count * 12.0).round() as i32)
    }
    pub fn from_16th_note(count: i32) -> Self {
        Self(count * 6)
    }
    pub fn from_16th_note_float(count: f64) -> Self {
        Self((count * 6.0).round() as i32)
    }
    pub fn from_32nd_note(count: i32) -> Self {
        Self(count * 3)
    }
    pub fn from_32nd_note_float(count: f64) -> Self {
        Self((count * 3.0).round() as i32)
    }
    pub fn from_32nd_triplet_note(count: i32) -> Self {
        Self(count)
    }
    pub fn from_32nd_triplet_note_float(count: f64) -> Self {
        Self(count.round() as i32)
    }

    pub fn to_nanoseconds(&self, tempo: f64) -> u64 {
        length_triplet_32nd_to_nanoseconds(self.0, tempo)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub pitch: u8,
    pub start_time_ns: u64,
    pub length: NoteLength,
    pub phonemes: Vec<String>,
    pub language: Option<String>,
    pub language_dependent_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposeOptions {
    pub tempo: f64,
    pub beat: String,
    pub key_signature: String,
    pub phrase_count: usize,
}

impl Default for ComposeOptions {
    fn default() -> Self {
        Self {
            tempo: 120.0,
            beat: "4/4".to_string(),
            key_signature: "0".to_string(),
            phrase_count: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeSignature {
    pub numerator: u8,
    pub denominator: u8,
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub notes: Vec<Note>,
    pub tempo: f64,
    pub time_signatures: Vec<TimeSignature>,
}

impl Default for Score {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            tempo: 120.0,
            time_signatures: vec![TimeSignature::default()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    TemplateParse(String),
    EmptyPhonemes { note_index: usize },
    InvalidPitch(String),
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TemplateParse(e) => write!(f, "failed to parse template label: {e}"),
            Self::EmptyPhonemes { note_index } => {
                write!(f, "note at index {note_index} has no phonemes")
            }
            Self::InvalidPitch(p) => write!(f, "invalid pitch label: {p}"),
        }
    }
}

impl std::error::Error for ComposeError {}

pub fn xx_as_none(value: &str) -> Option<&str> {
    if value == XX {
        None
    } else {
        Some(value)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct TimedLabel {
    pub label: Label,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
}

impl std::ops::Deref for TimedLabel {
    type Target = Label;

    fn deref(&self) -> &Self::Target {
        &self.label
    }
}

impl std::fmt::Debug for TimedLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.label.fmt(f)
    }
}

pub fn compose_labels_from_score(score: &Score) -> Result<Vec<TimedLabel>, ComposeError> {
    let time_signature = score
        .time_signatures
        .first()
        .cloned()
        .unwrap_or_else(TimeSignature::default);
    let options = ComposeOptions {
        tempo: score.tempo,
        beat: format!(
            "{}/{}",
            time_signature.numerator, time_signature.denominator
        ),
        key_signature: "0".to_string(),
        phrase_count: 1,
    };
    for (i, note) in score.notes.iter().enumerate() {
        if note.phonemes.is_empty() {
            return Err(ComposeError::EmptyPhonemes { note_index: i });
        }
    }

    let template = parse_label_line(TEMPLATE_LABEL_LINE)
        .map_err(|e| ComposeError::TemplateParse(e.to_string()))?;
    let points = flatten_points(&score.notes);
    let mut labels = Vec::with_capacity(points.len());
    let note_time_ranges_ns = compute_note_time_ranges_ns(&score.notes, options.tempo);

    for point in &points {
        let mut label = template.clone();
        fill_phoneme_context(&mut label, &points, point.index);
        fill_syllable_contexts(&mut label, &score.notes, point.note_index);
        fill_note_contexts(&mut label, &score.notes, point.note_index, &options);
        fill_phrase_and_song_contexts(&mut label, &score.notes, &options);
        let (note_start_ns, note_end_ns) = note_time_ranges_ns[point.note_index];
        let phoneme_count = score.notes[point.note_index].phonemes.len().max(1) as u64;
        let phoneme_index = point.phoneme_index as u64;
        let note_span_ns = note_end_ns.saturating_sub(note_start_ns);
        let start_time_ns =
            note_start_ns + note_span_ns.saturating_mul(phoneme_index) / phoneme_count;
        let end_time_ns = note_start_ns
            + note_span_ns.saturating_mul(phoneme_index.saturating_add(1)) / phoneme_count;
        labels.push(TimedLabel {
            label,
            start_time_ns,
            end_time_ns,
        });
    }

    Ok(labels)
}

fn compute_note_time_ranges_ns(notes: &[Note], tempo: f64) -> Vec<(u64, u64)> {
    let mut current_ns: u128 = 0;
    let mut ranges = Vec::with_capacity(notes.len());
    for note in notes {
        let length_triplet_32nd: i32 = note.length.into();
        let note_duration_ns =
            length_triplet_32nd_to_nanoseconds(length_triplet_32nd, tempo) as u128;
        // Keep legacy sequential behavior when start_time is 0, but honor explicit note start.
        let start_ns = current_ns.max(note.start_time_ns as u128);
        let end_ns = start_ns.saturating_add(note_duration_ns);
        ranges.push((
            start_ns.min(u64::MAX as u128) as u64,
            end_ns.min(u64::MAX as u128) as u64,
        ));
        current_ns = end_ns;
    }
    ranges
}

pub fn labels_to_notes(labels: &[Label]) -> Result<Vec<Note>, ComposeError> {
    Ok(labels_to_score(labels)?.notes)
}

pub fn labels_to_score(labels: &[Label]) -> Result<Score, ComposeError> {
    if labels.is_empty() {
        return Ok(Score::default());
    }

    let tempo = labels
        .iter()
        .find_map(|l| l.curr_note.tempo.as_option())
        .and_then(|t| t.parse::<i32>().ok())
        .unwrap_or(120)
        .into();

    let time_signature = labels
        .iter()
        .find_map(|l| l.curr_note.beat.as_option())
        .and_then(parse_time_signature)
        .unwrap_or_default();

    let mut notes = vec![Note {
        pitch: 60,
        start_time_ns: 0,
        length: NoteLength::from_4th_note(100),
        language: Some("JPN".to_string()),
        language_dependent_context: Some("p".to_string()),
        phonemes: vec!["pau".to_string()],
    }];
    let mut current_group_key: Option<String> = None;

    for label in labels {
        let group_key = label
            .curr_syllable
            .note_position_forward
            .as_option()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("g{}", notes.len()));

        if current_group_key.as_deref() != Some(group_key.as_str()) {
            let pitch_label = label.curr_note.absolute_pitch.as_option();
            let pitch = match pitch_label {
                Some(p) => {
                    note_name_to_midi(p).ok_or_else(|| ComposeError::InvalidPitch(p.to_string()))?
                }
                None => 60,
            };
            let start_time = notes
                .last()
                .map(|prev| {
                    prev.start_time_ns
                        .saturating_add(length_triplet_32nd_to_nanoseconds(
                            prev.length.into(),
                            tempo,
                        ))
                })
                .unwrap_or(0);

            notes.push(Note {
                pitch,
                start_time_ns: start_time,
                length: label
                    .curr_note
                    .length_triplet_32nd
                    .as_option()
                    .and_then(|v| v.parse::<i32>().ok())
                    .map(NoteLength::from_32nd_triplet_note)
                    .unwrap_or(NoteLength::from_32nd_triplet_note(1)),
                phonemes: Vec::new(),
                language: label
                    .curr_syllable
                    .language
                    .as_option()
                    .map(ToString::to_string),
                language_dependent_context: label
                    .curr_syllable
                    .language_dependent_context
                    .as_option()
                    .map(ToString::to_string),
            });
            current_group_key = Some(group_key);
        }

        if let Some(last) = notes.last_mut() {
            if let Some(p) = label.phoneme.phoneme_id_current.as_option() {
                last.phonemes.push(p.to_string());
            }
        }
    }
    notes.push(Note {
        pitch: 60,
        start_time_ns: notes
            .last()
            .map(|prev| {
                prev.start_time_ns
                    .saturating_add(length_triplet_32nd_to_nanoseconds(
                        prev.length.into(),
                        tempo,
                    ))
            })
            .unwrap_or(0),
        language_dependent_context: Some("p".to_string()),
        phonemes: vec!["pau".to_string()],
        language: Some("JPN".to_string()),
        length: NoteLength::from_4th_note(100),
    });

    Ok(Score {
        notes,
        tempo,
        time_signatures: vec![time_signature],
    })
}

#[derive(Debug, Clone)]
struct Point {
    index: usize,
    note_index: usize,
    phoneme_index: usize,
    symbol: String,
}

fn flatten_points(notes: &[Note]) -> Vec<Point> {
    let mut points = Vec::new();
    for (note_index, note) in notes.iter().enumerate() {
        for (phoneme_index, symbol) in note.phonemes.iter().enumerate() {
            points.push(Point {
                index: points.len(),
                note_index,
                phoneme_index,
                symbol: symbol.clone(),
            });
        }
    }
    points
}

fn fill_phoneme_context(label: &mut Label, points: &[Point], index: usize) {
    let current = &points[index];
    let current_count = points
        .iter()
        .filter(|p| p.note_index == current.note_index)
        .count();

    label.phoneme.language_independent_phoneme_id = current.symbol.clone().into();
    label.phoneme.phoneme_id_two_before = symbol_at(points, index, -2).to_string().into();
    label.phoneme.phoneme_id_previous = symbol_at(points, index, -1).to_string().into();
    label.phoneme.phoneme_id_current = current.symbol.clone().into();
    label.phoneme.phoneme_id_next = symbol_at(points, index, 1).to_string().into();
    label.phoneme.phoneme_id_two_after = symbol_at(points, index, 2).to_string().into();
    label.phoneme.phoneme_flag_two_before = XX.into();
    label.phoneme.phoneme_flag_before = XX.into();
    label.phoneme.phoneme_flag_current = "00".into();
    label.phoneme.phoneme_flag_next = "00".into();
    label.phoneme.phoneme_flag_two_after = "00".into();
    label.phoneme.syllable_phoneme_position_forward =
        (current.phoneme_index + 1).to_string().into();
    label.phoneme.syllable_phoneme_position_backward =
        (current_count - current.phoneme_index).to_string().into();
    label.phoneme.distance_from_prev_vowel = XX.into();
    label.phoneme.distance_to_next_vowel = XX.into();
    label.phoneme.reserved = XX.into();
}

fn fill_syllable_contexts(label: &mut Label, notes: &[Note], note_index: usize) {
    fill_syllable(
        &mut label.prev_syllable.phoneme_count,
        &mut label.prev_syllable.note_position_forward,
        &mut label.prev_syllable.note_position_backward,
        &mut label.prev_syllable.language,
        &mut label.prev_syllable.language_dependent_context,
        notes,
        note_index.checked_sub(1),
    );
    fill_syllable(
        &mut label.curr_syllable.phoneme_count,
        &mut label.curr_syllable.note_position_forward,
        &mut label.curr_syllable.note_position_backward,
        &mut label.curr_syllable.language,
        &mut label.curr_syllable.language_dependent_context,
        notes,
        Some(note_index),
    );
    fill_syllable(
        &mut label.next_syllable.phoneme_count,
        &mut label.next_syllable.note_position_forward,
        &mut label.next_syllable.note_position_backward,
        &mut label.next_syllable.language,
        &mut label.next_syllable.language_dependent_context,
        notes,
        note_index.checked_add(1).filter(|i| *i < notes.len()),
    );
}

fn fill_note_contexts(
    label: &mut Label,
    notes: &[Note],
    note_index: usize,
    options: &ComposeOptions,
) {
    fill_note(
        &mut label.prev_note.absolute_pitch,
        &mut label.prev_note.relative_pitch,
        &mut label.prev_note.key_signature,
        &mut label.prev_note.beat,
        &mut label.prev_note.tempo,
        &mut label.prev_note.length_syllable,
        &mut label.prev_note.length_centisecond,
        &mut label.prev_note.length_triplet_32nd,
        &mut label.prev_note.reserved,
        notes,
        note_index.checked_sub(1),
        options,
    );
    fill_note(
        &mut label.curr_note.absolute_pitch,
        &mut label.curr_note.relative_pitch,
        &mut label.curr_note.key_signature,
        &mut label.curr_note.beat,
        &mut label.curr_note.tempo,
        &mut label.curr_note.length_syllable,
        &mut label.curr_note.length_centisecond,
        &mut label.curr_note.length_triplet_32nd,
        &mut label.curr_note.reserved,
        notes,
        Some(note_index),
        options,
    );
    fill_note(
        &mut label.next_note.absolute_pitch,
        &mut label.next_note.relative_pitch,
        &mut label.next_note.key_signature,
        &mut label.next_note.beat,
        &mut label.next_note.tempo,
        &mut label.next_note.length_syllable,
        &mut label.next_note.length_centisecond,
        &mut label.next_note.length_triplet_32nd,
        &mut label.next_note.reserved,
        notes,
        note_index.checked_add(1).filter(|i| *i < notes.len()),
        options,
    );

    label.curr_note.measure_note_position_note_forward = "1".into();
    label.curr_note.measure_note_position_note_backward = "1".into();
    label.curr_note.measure_note_position_centisecond_forward = "0".into();
    label.curr_note.measure_note_position_centisecond_backward = "0".into();
    label.curr_note.measure_note_position_triplet_32nd_forward = "0".into();
    label.curr_note.measure_note_position_triplet_32nd_backward = "0".into();
    label.curr_note.measure_note_position_percent_forward = "0".into();
    label.curr_note.measure_note_position_percent_backward = "100".into();

    label.curr_note.phrase_note_position_note_forward = XX.into();
    label.curr_note.phrase_note_position_note_backward = XX.into();
    label.curr_note.phrase_note_position_centisecond_forward = XX.into();
    label.curr_note.phrase_note_position_centisecond_backward = XX.into();
    label.curr_note.phrase_note_position_triplet_32nd_forward = XX.into();
    label.curr_note.phrase_note_position_triplet_32nd_backward = XX.into();
    label.curr_note.phrase_note_position_percent_forward = XX.into();
    label.curr_note.phrase_note_position_percent_backward = XX.into();
    label.curr_note.slur_with_previous = "0".into();
    label.curr_note.slur_with_next = "0".into();
    label.curr_note.dynamic_mark = "n".into();
    label.curr_note.distance_to_next_accent_note = XX.into();
    label.curr_note.distance_to_previous_accent_note = XX.into();
    label.curr_note.distance_to_next_accent_centisecond = XX.into();
    label.curr_note.distance_to_previous_accent_centisecond = XX.into();
    label.curr_note.distance_to_next_accent_triplet_32nd = XX.into();
    label.curr_note.distance_to_previous_accent_triplet_32nd = XX.into();
    label.curr_note.distance_to_next_staccato_note = XX.into();
    label.curr_note.distance_to_previous_staccato_note = XX.into();
    label.curr_note.distance_to_next_staccato_centisecond = XX.into();
    label.curr_note.distance_to_previous_staccato_centisecond = XX.into();
    label.curr_note.distance_to_next_staccato_triplet_32nd = XX.into();
    label.curr_note.distance_to_previous_staccato_triplet_32nd = XX.into();
    label.curr_note.crescendo_position_note_forward = XX.into();
    label.curr_note.crescendo_position_note_backward = XX.into();
    label.curr_note.crescendo_position_second_forward = XX.into();
    label.curr_note.crescendo_position_second_backward = XX.into();
    label.curr_note.crescendo_position_triplet_32nd_forward = XX.into();
    label.curr_note.crescendo_position_triplet_32nd_backward = XX.into();
    label.curr_note.crescendo_position_percent_forward = XX.into();
    label.curr_note.crescendo_position_percent_backward = XX.into();
    label.curr_note.decrescendo_position_note_forward = XX.into();
    label.curr_note.decrescendo_position_note_backward = XX.into();
    label.curr_note.decrescendo_position_second_forward = XX.into();
    label.curr_note.decrescendo_position_second_backward = XX.into();
    label.curr_note.decrescendo_position_triplet_32nd_forward = XX.into();
    label.curr_note.decrescendo_position_triplet_32nd_backward = XX.into();
    label.curr_note.decrescendo_position_percent_forward = XX.into();
    label.curr_note.decrescendo_position_percent_backward = XX.into();
    label.curr_note.pitch_difference_from_previous_note = XX.into();
    label.curr_note.pitch_difference_to_next_note = notes
        .get(note_index + 1)
        .map(|next_note| format_pitch_difference(notes[note_index].pitch, next_note.pitch).into())
        .unwrap_or_else(|| XX.into());
    label.curr_note.reserved_2 = XX.into();
    label.curr_note.reserved_3 = XX.into();
}

fn fill_phrase_and_song_contexts(label: &mut Label, notes: &[Note], options: &ComposeOptions) {
    let total_phonemes = notes.iter().map(|n| n.phonemes.len()).sum::<usize>();

    label.prev_phrase.syllable_count = XX.into();
    label.prev_phrase.phoneme_count = XX.into();
    label.curr_phrase.syllable_count = notes.len().to_string().into();
    label.curr_phrase.phoneme_count = total_phonemes.to_string().into();
    label.next_phrase.syllable_count = XX.into();
    label.next_phrase.phoneme_count = XX.into();
    label.song.syllable_per_measure = XX.into();
    label.song.phoneme_per_measure = XX.into();
    label.song.phrase_count = options.phrase_count.to_string().into();
}

fn fill_syllable(
    phoneme_count: &mut LabelValue,
    note_position_forward: &mut LabelValue,
    note_position_backward: &mut LabelValue,
    language: &mut LabelValue,
    language_dependent_context: &mut LabelValue,
    notes: &[Note],
    idx: Option<usize>,
) {
    if let Some(i) = idx {
        *phoneme_count = notes[i].phonemes.len().to_string().into();
        *note_position_forward = (i + 1).to_string().into();
        *note_position_backward = (notes.len() - i).to_string().into();
        *language = notes[i]
            .language
            .clone()
            .unwrap_or_else(|| XX.to_string())
            .into();
        *language_dependent_context = notes[i]
            .language_dependent_context
            .clone()
            .unwrap_or_else(|| XX.to_string())
            .into();
    } else {
        *phoneme_count = XX.into();
        *note_position_forward = XX.into();
        *note_position_backward = XX.into();
        *language = XX.into();
        *language_dependent_context = XX.into();
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_note(
    absolute_pitch: &mut LabelValue,
    relative_pitch: &mut LabelValue,
    key_signature: &mut LabelValue,
    beat: &mut LabelValue,
    tempo: &mut LabelValue,
    length_syllable: &mut LabelValue,
    length_centisecond: &mut LabelValue,
    length_triplet_32nd: &mut LabelValue,
    reserved: &mut LabelValue,
    notes: &[Note],
    idx: Option<usize>,
    options: &ComposeOptions,
) {
    if let Some(i) = idx {
        let length_triplet_32nd_value: i32 = notes[i].length.into();
        *absolute_pitch = midi_to_note_name(notes[i].pitch).into();
        *relative_pitch = notes[i].pitch.rem_euclid(12).to_string().into();
        *key_signature = options.key_signature.clone().into();
        *beat = options.beat.clone().into();
        *tempo = options.tempo.to_string().into();
        *length_syllable = "1".into();
        *length_centisecond =
            length_triplet_32nd_to_centiseconds(length_triplet_32nd_value, options.tempo)
                .to_string()
                .into();
        *length_triplet_32nd = length_triplet_32nd_value.to_string().into();
        *reserved = XX.into();
    } else {
        *absolute_pitch = XX.into();
        *relative_pitch = XX.into();
        *key_signature = XX.into();
        *beat = XX.into();
        *tempo = XX.into();
        *length_syllable = XX.into();
        *length_centisecond = XX.into();
        *length_triplet_32nd = XX.into();
        *reserved = XX.into();
    }
}

fn length_triplet_32nd_to_centiseconds(length_triplet_32nd: i32, tempo: f64) -> i32 {
    if tempo <= 0.0 {
        return 0;
    }
    // 24 triplet-32nd notes = 1 quarter note.
    // centiseconds = length * (60 * 100) / (24 * tempo) = length * 250 / tempo
    (length_triplet_32nd * 250 + (tempo / 2.0) as i32) / tempo as i32
}

fn length_triplet_32nd_to_nanoseconds(length_triplet_32nd: i32, tempo: f64) -> u64 {
    if tempo <= 0.0 {
        return 0;
    }
    let ns = (length_triplet_32nd.max(0) as f64) * 2_500_000_000.0 / tempo;
    if !ns.is_finite() || ns <= 0.0 {
        0
    } else if ns >= u64::MAX as f64 {
        u64::MAX
    } else {
        ns.round() as u64
    }
}

fn format_pitch_difference(current_pitch: u8, target_pitch: u8) -> String {
    let diff = target_pitch as i16 - current_pitch as i16;
    if diff >= 0 {
        format!("p{diff}")
    } else {
        format!("m{}", -diff)
    }
}

fn symbol_at(points: &[Point], index: usize, offset: isize) -> &str {
    let shifted = index as isize + offset;
    if shifted < 0 || shifted >= points.len() as isize {
        XX
    } else {
        points[shifted as usize].symbol.as_str()
    }
}

fn midi_to_note_name(pitch: u8) -> String {
    const NOTE_NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let pc = pitch.rem_euclid(12) as usize;
    let octave = pitch.div_euclid(12) - 1;
    format!("{}{}", NOTE_NAMES[pc], octave)
}

fn note_name_to_midi(name: &str) -> Option<u8> {
    if name.len() < 2 {
        return None;
    }

    let (head, octave_str) = if name.as_bytes().get(1) == Some(&b'#') {
        (&name[..2], &name[2..])
    } else {
        (&name[..1], &name[1..])
    };

    let pitch_class = match head {
        "C" => 0,
        "C#" => 1,
        "D" => 2,
        "D#" => 3,
        "E" => 4,
        "F" => 5,
        "F#" => 6,
        "G" => 7,
        "G#" => 8,
        "A" => 9,
        "A#" => 10,
        "B" => 11,
        _ => return None,
    };

    let octave = octave_str.parse::<i32>().ok()?;
    let midi = (octave + 1) * 12 + pitch_class;
    if !(0..=127).contains(&midi) {
        None
    } else {
        Some(midi as u8)
    }
}

fn parse_time_signature(value: &str) -> Option<TimeSignature> {
    let (num, den) = value.split_once('/')?;
    let numerator = num.parse::<u8>().ok()?;
    let denominator = den.parse::<u8>().ok()?;
    if numerator == 0 || denominator == 0 {
        return None;
    }
    Some(TimeSignature {
        numerator,
        denominator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xx_is_treated_as_none() {
        assert_eq!(xx_as_none("xx"), None);
        assert_eq!(xx_as_none("0"), Some("0"));
    }

    #[test]
    fn score_to_labels() {
        let score = Score {
            notes: vec![
                Note {
                    pitch: 60,
                    start_time_ns: 0,
                    length: NoteLength::from_4th_note(1),
                    phonemes: vec!["pau".to_string()],
                    language: Some("JPN".to_string()),
                    language_dependent_context: Some("0".to_string()),
                },
                Note {
                    pitch: 60,
                    start_time_ns: 0,
                    length: NoteLength::from_4th_note(1),
                    phonemes: vec!["p".to_string(), "a".to_string()],
                    language: Some("JPN".to_string()),
                    language_dependent_context: Some("0".to_string()),
                },
                Note {
                    pitch: 60,
                    start_time_ns: 0,
                    length: NoteLength::from_4th_note(1),
                    phonemes: vec!["pau".to_string()],
                    language: Some("JPN".to_string()),
                    language_dependent_context: Some("0".to_string()),
                },
            ],
            tempo: 140.0,
            time_signatures: vec![TimeSignature {
                numerator: 3,
                denominator: 4,
            }],
        };
        let labels = compose_labels_from_score(&score).expect("compose should succeed");
        assert_eq!(labels.len(), 4);
        assert_eq!(
            labels[0].phoneme.phoneme_id_current.as_option(),
            Some("pau")
        );
        assert_eq!(labels[1].phoneme.phoneme_id_current.as_option(), Some("p"));
        assert_eq!(labels[2].phoneme.phoneme_id_current.as_option(), Some("a"));
        assert_eq!(
            labels[3].phoneme.phoneme_id_current.as_option(),
            Some("pau")
        );
        assert_eq!(labels[0].curr_syllable.language.as_option(), Some("JPN"));
        assert_eq!(
            labels[0]
                .curr_syllable
                .language_dependent_context
                .as_option(),
            Some("0")
        );

        insta::assert_debug_snapshot!(labels);
    }

    #[test]
    fn labels_to_score_roundtrip() {
        let score = Score {
            notes: vec![
                Note {
                    pitch: 60,
                    start_time_ns: 0,
                    length: NoteLength::from_4th_note(1),
                    phonemes: vec!["p".to_string(), "a".to_string()],
                    language: Some("JPN".to_string()),
                    language_dependent_context: Some("0".to_string()),
                },
                Note {
                    pitch: 62,
                    start_time_ns: 0,
                    length: NoteLength::from_4th_note(1),
                    phonemes: vec!["r".to_string()],
                    language: Some("JPN".to_string()),
                    language_dependent_context: Some("0".to_string()),
                },
            ],
            tempo: 140.0,
            time_signatures: vec![TimeSignature {
                numerator: 3,
                denominator: 4,
            }],
        };

        let labels = compose_labels_from_score(&score).expect("compose should succeed");
        let plain_labels: Vec<Label> = labels.iter().map(|l| l.label.clone()).collect();
        let recovered = labels_to_score(&plain_labels).expect("recover should succeed");

        assert_eq!(recovered.tempo, 140.0);
        assert_eq!(
            recovered.time_signatures,
            vec![TimeSignature {
                numerator: 3,
                denominator: 4
            }]
        );
        assert_eq!(recovered.notes.len(), 2);
        assert_eq!(
            recovered.notes[0].phonemes,
            vec!["p".to_string(), "a".to_string()]
        );
        assert_eq!(recovered.notes[1].phonemes, vec!["r".to_string()]);
    }

    #[test]
    fn triplet_length_to_centiseconds() {
        assert_eq!(length_triplet_32nd_to_centiseconds(24, 100.0), 60);
        assert_eq!(length_triplet_32nd_to_centiseconds(96, 100.0), 240);
        assert_eq!(length_triplet_32nd_to_centiseconds(24, 140.0), 43);
    }

    #[test]
    fn pitch_difference_format() {
        assert_eq!(format_pitch_difference(60, 62), "p2");
        assert_eq!(format_pitch_difference(62, 60), "m2");
        assert_eq!(format_pitch_difference(60, 60), "p0");
    }
}
