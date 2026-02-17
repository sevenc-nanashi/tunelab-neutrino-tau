use lazy_regex::{regex, Regex};

#[derive(Clone, PartialEq, Eq, Default)]
pub struct LabelValue(String);

impl std::fmt::Debug for LabelValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::fmt::Display for LabelValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl From<&str> for LabelValue {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for LabelValue {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl PartialEq<&str> for LabelValue {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub phoneme: PhonemeContext,
    pub prev_syllable: PrevSyllable,
    pub curr_syllable: CurrSyllable,
    pub next_syllable: NextSyllable,
    pub prev_note: PrevNote,
    pub curr_note: CurrNote,
    pub next_note: NextNote,
    pub prev_phrase: PrevPhrase,
    pub curr_phrase: CurrPhrase,
    pub next_phrase: NextPhrase,
    pub song: SongContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhonemeContext {
    pub language_independent_phoneme_id: LabelValue,
    pub phoneme_id_two_before: LabelValue,
    pub phoneme_id_previous: LabelValue,
    pub phoneme_id_current: LabelValue,
    pub phoneme_id_next: LabelValue,
    pub phoneme_id_two_after: LabelValue,
    pub phoneme_flag_two_before: LabelValue,
    pub phoneme_flag_before: LabelValue,
    pub phoneme_flag_current: LabelValue,
    pub phoneme_flag_next: LabelValue,
    pub phoneme_flag_two_after: LabelValue,
    pub syllable_phoneme_position_forward: LabelValue,
    pub syllable_phoneme_position_backward: LabelValue,
    pub distance_from_prev_vowel: LabelValue,
    pub distance_to_next_vowel: LabelValue,
    pub reserved: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrevSyllable {
    pub phoneme_count: LabelValue,
    pub note_position_forward: LabelValue,
    pub note_position_backward: LabelValue,
    pub language: LabelValue,
    pub language_dependent_context: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrSyllable {
    pub phoneme_count: LabelValue,
    pub note_position_forward: LabelValue,
    pub note_position_backward: LabelValue,
    pub language: LabelValue,
    pub language_dependent_context: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextSyllable {
    pub phoneme_count: LabelValue,
    pub note_position_forward: LabelValue,
    pub note_position_backward: LabelValue,
    pub language: LabelValue,
    pub language_dependent_context: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrevNote {
    pub absolute_pitch: LabelValue,
    pub relative_pitch: LabelValue,
    pub key_signature: LabelValue,
    pub beat: LabelValue,
    pub tempo: LabelValue,
    pub length_syllable: LabelValue,
    pub length_centisecond: LabelValue,
    pub length_triplet_32nd: LabelValue,
    pub reserved: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrNote {
    pub absolute_pitch: LabelValue,
    pub relative_pitch: LabelValue,
    pub key_signature: LabelValue,
    pub beat: LabelValue,
    pub tempo: LabelValue,
    pub length_syllable: LabelValue,
    pub length_centisecond: LabelValue,
    pub length_triplet_32nd: LabelValue,
    pub reserved: LabelValue,
    pub measure_note_position_note_forward: LabelValue,
    pub measure_note_position_note_backward: LabelValue,
    pub measure_note_position_centisecond_forward: LabelValue,
    pub measure_note_position_centisecond_backward: LabelValue,
    pub measure_note_position_triplet_32nd_forward: LabelValue,
    pub measure_note_position_triplet_32nd_backward: LabelValue,
    pub measure_note_position_percent_forward: LabelValue,
    pub measure_note_position_percent_backward: LabelValue,
    pub phrase_note_position_note_forward: LabelValue,
    pub phrase_note_position_note_backward: LabelValue,
    pub phrase_note_position_centisecond_forward: LabelValue,
    pub phrase_note_position_centisecond_backward: LabelValue,
    pub phrase_note_position_triplet_32nd_forward: LabelValue,
    pub phrase_note_position_triplet_32nd_backward: LabelValue,
    pub phrase_note_position_percent_forward: LabelValue,
    pub phrase_note_position_percent_backward: LabelValue,
    pub slur_with_previous: LabelValue,
    pub slur_with_next: LabelValue,
    pub dynamic_mark: LabelValue,
    pub distance_to_next_accent_note: LabelValue,
    pub distance_to_previous_accent_note: LabelValue,
    pub distance_to_next_accent_centisecond: LabelValue,
    pub distance_to_previous_accent_centisecond: LabelValue,
    pub distance_to_next_accent_triplet_32nd: LabelValue,
    pub distance_to_previous_accent_triplet_32nd: LabelValue,
    pub distance_to_next_staccato_note: LabelValue,
    pub distance_to_previous_staccato_note: LabelValue,
    pub distance_to_next_staccato_centisecond: LabelValue,
    pub distance_to_previous_staccato_centisecond: LabelValue,
    pub distance_to_next_staccato_triplet_32nd: LabelValue,
    pub distance_to_previous_staccato_triplet_32nd: LabelValue,
    pub crescendo_position_note_forward: LabelValue,
    pub crescendo_position_note_backward: LabelValue,
    pub crescendo_position_second_forward: LabelValue,
    pub crescendo_position_second_backward: LabelValue,
    pub crescendo_position_triplet_32nd_forward: LabelValue,
    pub crescendo_position_triplet_32nd_backward: LabelValue,
    pub crescendo_position_percent_forward: LabelValue,
    pub crescendo_position_percent_backward: LabelValue,
    pub decrescendo_position_note_forward: LabelValue,
    pub decrescendo_position_note_backward: LabelValue,
    pub decrescendo_position_second_forward: LabelValue,
    pub decrescendo_position_second_backward: LabelValue,
    pub decrescendo_position_triplet_32nd_forward: LabelValue,
    pub decrescendo_position_triplet_32nd_backward: LabelValue,
    pub decrescendo_position_percent_forward: LabelValue,
    pub decrescendo_position_percent_backward: LabelValue,
    pub pitch_difference_from_previous_note: LabelValue,
    pub pitch_difference_to_next_note: LabelValue,
    pub reserved_2: LabelValue,
    pub reserved_3: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextNote {
    pub absolute_pitch: LabelValue,
    pub relative_pitch: LabelValue,
    pub key_signature: LabelValue,
    pub beat: LabelValue,
    pub tempo: LabelValue,
    pub length_syllable: LabelValue,
    pub length_centisecond: LabelValue,
    pub length_triplet_32nd: LabelValue,
    pub reserved: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrevPhrase {
    pub syllable_count: LabelValue,
    pub phoneme_count: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrPhrase {
    pub syllable_count: LabelValue,
    pub phoneme_count: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NextPhrase {
    pub syllable_count: LabelValue,
    pub phoneme_count: LabelValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongContext {
    pub syllable_per_measure: LabelValue,
    pub phoneme_per_measure: LabelValue,
    pub phrase_count: LabelValue,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub section: &'static str,
    pub message: LabelValue,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.section, self.message)
    }
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}^{}-{}+{}={}_{}%{}^{}_{}~{}-{}!{}[{}${}]{}\
/A:{}-{}-{}@{}~{}\
/B:{}_{}_{}@{}|{}\
/C:{}+{}+{}@{}&{}\
/D:{}!{}#{}${}%{}|{}&{};{}-{}\
/E:{}]{}^{}={}~{}!{}@{}#{}+{}]{}${}|{}[{}&{}]{}={}^{}~{}#{}_{};{}${}&{}%{}[{}|{}]{}-{}^{}+{}~{}={}@{}${}!{}%{}#{}|{}|{}-{}&{}&{}+{}[{};{}]{};{}~{}~{}^{}^{}@{}[{}#{}={}!{}~{}+{}!{}^{}\
/F:{}#{}#{}-{}${}${}+{}%{};{}\
/G:{}_{}\
/H:{}_{}\
/I:{}_{}\
/J:{}~{}@{}",
            self.phoneme.language_independent_phoneme_id,
            self.phoneme.phoneme_id_two_before,
            self.phoneme.phoneme_id_previous,
            self.phoneme.phoneme_id_current,
            self.phoneme.phoneme_id_next,
            self.phoneme.phoneme_id_two_after,
            self.phoneme.phoneme_flag_two_before,
            self.phoneme.phoneme_flag_before,
            self.phoneme.phoneme_flag_current,
            self.phoneme.phoneme_flag_next,
            self.phoneme.phoneme_flag_two_after,
            self.phoneme.syllable_phoneme_position_forward,
            self.phoneme.syllable_phoneme_position_backward,
            self.phoneme.distance_from_prev_vowel,
            self.phoneme.distance_to_next_vowel,
            self.phoneme.reserved,
            self.prev_syllable.phoneme_count,
            self.prev_syllable.note_position_forward,
            self.prev_syllable.note_position_backward,
            self.prev_syllable.language,
            self.prev_syllable.language_dependent_context,
            self.curr_syllable.phoneme_count,
            self.curr_syllable.note_position_forward,
            self.curr_syllable.note_position_backward,
            self.curr_syllable.language,
            self.curr_syllable.language_dependent_context,
            self.next_syllable.phoneme_count,
            self.next_syllable.note_position_forward,
            self.next_syllable.note_position_backward,
            self.next_syllable.language,
            self.next_syllable.language_dependent_context,
            self.prev_note.absolute_pitch,
            self.prev_note.relative_pitch,
            self.prev_note.key_signature,
            self.prev_note.beat,
            self.prev_note.tempo,
            self.prev_note.length_syllable,
            self.prev_note.length_centisecond,
            self.prev_note.length_triplet_32nd,
            self.prev_note.reserved,
            self.curr_note.absolute_pitch,
            self.curr_note.relative_pitch,
            self.curr_note.key_signature,
            self.curr_note.beat,
            self.curr_note.tempo,
            self.curr_note.length_syllable,
            self.curr_note.length_centisecond,
            self.curr_note.length_triplet_32nd,
            self.curr_note.reserved,
            self.curr_note.measure_note_position_note_forward,
            self.curr_note.measure_note_position_note_backward,
            self.curr_note.measure_note_position_centisecond_forward,
            self.curr_note.measure_note_position_centisecond_backward,
            self.curr_note.measure_note_position_triplet_32nd_forward,
            self.curr_note.measure_note_position_triplet_32nd_backward,
            self.curr_note.measure_note_position_percent_forward,
            self.curr_note.measure_note_position_percent_backward,
            self.curr_note.phrase_note_position_note_forward,
            self.curr_note.phrase_note_position_note_backward,
            self.curr_note.phrase_note_position_centisecond_forward,
            self.curr_note.phrase_note_position_centisecond_backward,
            self.curr_note.phrase_note_position_triplet_32nd_forward,
            self.curr_note.phrase_note_position_triplet_32nd_backward,
            self.curr_note.phrase_note_position_percent_forward,
            self.curr_note.phrase_note_position_percent_backward,
            self.curr_note.slur_with_previous,
            self.curr_note.slur_with_next,
            self.curr_note.dynamic_mark,
            self.curr_note.distance_to_next_accent_note,
            self.curr_note.distance_to_previous_accent_note,
            self.curr_note.distance_to_next_accent_centisecond,
            self.curr_note.distance_to_previous_accent_centisecond,
            self.curr_note.distance_to_next_accent_triplet_32nd,
            self.curr_note.distance_to_previous_accent_triplet_32nd,
            self.curr_note.distance_to_next_staccato_note,
            self.curr_note.distance_to_previous_staccato_note,
            self.curr_note.distance_to_next_staccato_centisecond,
            self.curr_note.distance_to_previous_staccato_centisecond,
            self.curr_note.distance_to_next_staccato_triplet_32nd,
            self.curr_note.distance_to_previous_staccato_triplet_32nd,
            self.curr_note.crescendo_position_note_forward,
            self.curr_note.crescendo_position_note_backward,
            self.curr_note.crescendo_position_second_forward,
            self.curr_note.crescendo_position_second_backward,
            self.curr_note.crescendo_position_triplet_32nd_forward,
            self.curr_note.crescendo_position_triplet_32nd_backward,
            self.curr_note.crescendo_position_percent_forward,
            self.curr_note.crescendo_position_percent_backward,
            self.curr_note.decrescendo_position_note_forward,
            self.curr_note.decrescendo_position_note_backward,
            self.curr_note.decrescendo_position_second_forward,
            self.curr_note.decrescendo_position_second_backward,
            self.curr_note.decrescendo_position_triplet_32nd_forward,
            self.curr_note.decrescendo_position_triplet_32nd_backward,
            self.curr_note.decrescendo_position_percent_forward,
            self.curr_note.decrescendo_position_percent_backward,
            self.curr_note.pitch_difference_from_previous_note,
            self.curr_note.pitch_difference_to_next_note,
            self.curr_note.reserved_2,
            self.curr_note.reserved_3,
            self.next_note.absolute_pitch,
            self.next_note.relative_pitch,
            self.next_note.key_signature,
            self.next_note.beat,
            self.next_note.tempo,
            self.next_note.length_syllable,
            self.next_note.length_centisecond,
            self.next_note.length_triplet_32nd,
            self.next_note.reserved,
            self.prev_phrase.syllable_count,
            self.prev_phrase.phoneme_count,
            self.curr_phrase.syllable_count,
            self.curr_phrase.phoneme_count,
            self.next_phrase.syllable_count,
            self.next_phrase.phoneme_count,
            self.song.syllable_per_measure,
            self.song.phoneme_per_measure,
            self.song.phrase_count
        )
    }
}

pub fn parse_label_line(line: &str) -> Result<Label, ParseError> {
    let re = regex!(
        r"^(.*?)/A:(.*?)/B:(.*?)/C:(.*?)/D:(.*?)/E:(.*?)/F:(.*?)/G:(.*?)/H:(.*?)/I:(.*?)/J:(.*)$"
    );
    let captures = re.captures(line).ok_or_else(|| ParseError {
        section: "root",
        message: "missing required /A..../J sections".to_string().into(),
    })?;
    let field = |index: usize| -> Result<&str, ParseError> {
        captures
            .get(index)
            .map(|m| m.as_str().trim())
            .ok_or_else(|| ParseError {
                section: "root",
                message: format!("capture {} not found", index).into(),
            })
    };

    let phoneme = parse_p(field(1)?)?;
    let prev_syllable = parse_a(field(2)?)?;
    let curr_syllable = parse_b(field(3)?)?;
    let next_syllable = parse_c(field(4)?)?;
    let prev_note = parse_d(field(5)?)?;
    let curr_note = parse_e(field(6)?)?;
    let next_note = parse_f(field(7)?)?;
    let prev_phrase = parse_g(field(8)?)?;
    let curr_phrase = parse_h(field(9)?)?;
    let next_phrase = parse_i(field(10)?)?;
    let song = parse_j(field(11)?)?;

    Ok(Label {
        phoneme,
        prev_syllable,
        curr_syllable,
        next_syllable,
        prev_note,
        curr_note,
        next_note,
        prev_phrase,
        curr_phrase,
        next_phrase,
        song,
    })
}

fn parse_p(input: &str) -> Result<PhonemeContext, ParseError> {
    let fields = capture_fields(
        regex!(
            r"^(.+?)@(.+?)\^(.+?)-(.+?)\+(.+?)=(.+?)_(.+?)%(.+?)\^(.+?)_(.+?)~(.+?)-(.+?)!(.+?)\[(.+?)\$(.+?)\](.+?)$"
        ),
        input,
        16,
        "P",
    )?;
    Ok(PhonemeContext {
        language_independent_phoneme_id: fields[0].clone(),
        phoneme_id_two_before: fields[1].clone(),
        phoneme_id_previous: fields[2].clone(),
        phoneme_id_current: fields[3].clone(),
        phoneme_id_next: fields[4].clone(),
        phoneme_id_two_after: fields[5].clone(),
        phoneme_flag_two_before: fields[6].clone(),
        phoneme_flag_before: fields[7].clone(),
        phoneme_flag_current: fields[8].clone(),
        phoneme_flag_next: fields[9].clone(),
        phoneme_flag_two_after: fields[10].clone(),
        syllable_phoneme_position_forward: fields[11].clone(),
        syllable_phoneme_position_backward: fields[12].clone(),
        distance_from_prev_vowel: fields[13].clone(),
        distance_to_next_vowel: fields[14].clone(),
        reserved: fields[15].clone(),
    })
}

fn parse_a(input: &str) -> Result<PrevSyllable, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)-(.+?)-(.+?)@(.+?)~(.+?)$"), input, 5, "A")?;
    Ok(PrevSyllable {
        phoneme_count: fields[0].clone(),
        note_position_forward: fields[1].clone(),
        note_position_backward: fields[2].clone(),
        language: fields[3].clone(),
        language_dependent_context: fields[4].clone(),
    })
}

fn parse_b(input: &str) -> Result<CurrSyllable, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)_(.+?)_(.+?)@(.+?)\|(.+?)$"), input, 5, "B")?;
    Ok(CurrSyllable {
        phoneme_count: fields[0].clone(),
        note_position_forward: fields[1].clone(),
        note_position_backward: fields[2].clone(),
        language: fields[3].clone(),
        language_dependent_context: fields[4].clone(),
    })
}

fn parse_c(input: &str) -> Result<NextSyllable, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)\+(.+?)\+(.+?)@(.+?)&(.+?)$"), input, 5, "C")?;
    Ok(NextSyllable {
        phoneme_count: fields[0].clone(),
        note_position_forward: fields[1].clone(),
        note_position_backward: fields[2].clone(),
        language: fields[3].clone(),
        language_dependent_context: fields[4].clone(),
    })
}

fn parse_d(input: &str) -> Result<PrevNote, ParseError> {
    let fields = capture_fields(
        regex!(r"^(.+?)!(.+?)#(.+?)\$(.+?)%(.+?)\|(.+?)&(.+?);(.+?)-(.+?)$"),
        input,
        9,
        "D",
    )?;
    Ok(PrevNote {
        absolute_pitch: fields[0].clone(),
        relative_pitch: fields[1].clone(),
        key_signature: fields[2].clone(),
        beat: fields[3].clone(),
        tempo: fields[4].clone(),
        length_syllable: fields[5].clone(),
        length_centisecond: fields[6].clone(),
        length_triplet_32nd: fields[7].clone(),
        reserved: fields[8].clone(),
    })
}

fn parse_e(input: &str) -> Result<CurrNote, ParseError> {
    let fields = capture_fields(
        regex!(
            r"^(.+?)\](.+?)\^(.+?)=(.+?)~(.+?)!(.+?)@(.+?)#(.+?)\+(.+?)\](.+?)\$(.+?)\|(.+?)\[(.+?)&(.+?)\](.+?)=(.+?)\^(.+?)~(.+?)#(.+?)_(.+?);(.+?)\$(.+?)&(.+?)%(.+?)\[(.+?)\|(.+?)\](.+?)-(.+?)\^(.+?)\+(.+?)~(.+?)=(.+?)@(.+?)\$(.+?)!(.+?)%(.+?)#(.+?)\|(.+?)\|(.+?)-(.+?)&(.+?)&(.+?)\+(.+?)\[(.+?);(.+?)\](.+?);(.+?)~(.+?)~(.+?)\^(.+?)\^(.+?)@(.+?)\[(.+?)#(.+?)=(.+?)!(.+?)~(.+?)\+(.+?)!(.+?)\^(.+?)$"
        ),
        input,
        60,
        "E",
    )?;
    Ok(CurrNote {
        absolute_pitch: fields[0].clone(),
        relative_pitch: fields[1].clone(),
        key_signature: fields[2].clone(),
        beat: fields[3].clone(),
        tempo: fields[4].clone(),
        length_syllable: fields[5].clone(),
        length_centisecond: fields[6].clone(),
        length_triplet_32nd: fields[7].clone(),
        reserved: fields[8].clone(),
        measure_note_position_note_forward: fields[9].clone(),
        measure_note_position_note_backward: fields[10].clone(),
        measure_note_position_centisecond_forward: fields[11].clone(),
        measure_note_position_centisecond_backward: fields[12].clone(),
        measure_note_position_triplet_32nd_forward: fields[13].clone(),
        measure_note_position_triplet_32nd_backward: fields[14].clone(),
        measure_note_position_percent_forward: fields[15].clone(),
        measure_note_position_percent_backward: fields[16].clone(),
        phrase_note_position_note_forward: fields[17].clone(),
        phrase_note_position_note_backward: fields[18].clone(),
        phrase_note_position_centisecond_forward: fields[19].clone(),
        phrase_note_position_centisecond_backward: fields[20].clone(),
        phrase_note_position_triplet_32nd_forward: fields[21].clone(),
        phrase_note_position_triplet_32nd_backward: fields[22].clone(),
        phrase_note_position_percent_forward: fields[23].clone(),
        phrase_note_position_percent_backward: fields[24].clone(),
        slur_with_previous: fields[25].clone(),
        slur_with_next: fields[26].clone(),
        dynamic_mark: fields[27].clone(),
        distance_to_next_accent_note: fields[28].clone(),
        distance_to_previous_accent_note: fields[29].clone(),
        distance_to_next_accent_centisecond: fields[30].clone(),
        distance_to_previous_accent_centisecond: fields[31].clone(),
        distance_to_next_accent_triplet_32nd: fields[32].clone(),
        distance_to_previous_accent_triplet_32nd: fields[33].clone(),
        distance_to_next_staccato_note: fields[34].clone(),
        distance_to_previous_staccato_note: fields[35].clone(),
        distance_to_next_staccato_centisecond: fields[36].clone(),
        distance_to_previous_staccato_centisecond: fields[37].clone(),
        distance_to_next_staccato_triplet_32nd: fields[38].clone(),
        distance_to_previous_staccato_triplet_32nd: fields[39].clone(),
        crescendo_position_note_forward: fields[40].clone(),
        crescendo_position_note_backward: fields[41].clone(),
        crescendo_position_second_forward: fields[42].clone(),
        crescendo_position_second_backward: fields[43].clone(),
        crescendo_position_triplet_32nd_forward: fields[44].clone(),
        crescendo_position_triplet_32nd_backward: fields[45].clone(),
        crescendo_position_percent_forward: fields[46].clone(),
        crescendo_position_percent_backward: fields[47].clone(),
        decrescendo_position_note_forward: fields[48].clone(),
        decrescendo_position_note_backward: fields[49].clone(),
        decrescendo_position_second_forward: fields[50].clone(),
        decrescendo_position_second_backward: fields[51].clone(),
        decrescendo_position_triplet_32nd_forward: fields[52].clone(),
        decrescendo_position_triplet_32nd_backward: fields[53].clone(),
        decrescendo_position_percent_forward: fields[54].clone(),
        decrescendo_position_percent_backward: fields[55].clone(),
        pitch_difference_from_previous_note: fields[56].clone(),
        pitch_difference_to_next_note: fields[57].clone(),
        reserved_2: fields[58].clone(),
        reserved_3: fields[59].clone(),
    })
}

fn parse_f(input: &str) -> Result<NextNote, ParseError> {
    let fields = capture_fields(
        regex!(
            r"^(.+?)#\s*(.+?)#\s*(.+?)-\s*(.+?)\$\s*(.+?)\$\s*(.+?)\+\s*(.+?)%\s*(.+?);\s*(.+?)$"
        ),
        input,
        9,
        "F",
    )?;
    Ok(NextNote {
        absolute_pitch: fields[0].clone(),
        relative_pitch: fields[1].clone(),
        key_signature: fields[2].clone(),
        beat: fields[3].clone(),
        tempo: fields[4].clone(),
        length_syllable: fields[5].clone(),
        length_centisecond: fields[6].clone(),
        length_triplet_32nd: fields[7].clone(),
        reserved: fields[8].clone(),
    })
}

fn parse_g(input: &str) -> Result<PrevPhrase, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)_(.+?)$"), input, 2, "G")?;
    Ok(PrevPhrase {
        syllable_count: fields[0].clone(),
        phoneme_count: fields[1].clone(),
    })
}

fn parse_h(input: &str) -> Result<CurrPhrase, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)_(.+?)$"), input, 2, "H")?;
    Ok(CurrPhrase {
        syllable_count: fields[0].clone(),
        phoneme_count: fields[1].clone(),
    })
}

fn parse_i(input: &str) -> Result<NextPhrase, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)_(.+?)$"), input, 2, "I")?;
    Ok(NextPhrase {
        syllable_count: fields[0].clone(),
        phoneme_count: fields[1].clone(),
    })
}

fn parse_j(input: &str) -> Result<SongContext, ParseError> {
    let fields = capture_fields(regex!(r"^(.+?)~(.+?)@_*(.+?)$"), input, 3, "J")?;
    Ok(SongContext {
        syllable_per_measure: fields[0].clone(),
        phoneme_per_measure: fields[1].clone(),
        phrase_count: fields[2].clone(),
    })
}

fn capture_fields(
    re: &Regex,
    input: &str,
    expected_fields: usize,
    section: &'static str,
) -> Result<Vec<LabelValue>, ParseError> {
    let captures = re.captures(input).ok_or_else(|| ParseError {
        section,
        message: format!("format mismatch: {input}").into(),
    })?;
    let mut fields = Vec::with_capacity(expected_fields);
    for i in 1..=expected_fields {
        let value = captures.get(i).ok_or_else(|| ParseError {
            section,
            message: format!("missing capture group {}", i).into(),
        })?;
        fields.push(LabelValue::from(value.as_str().trim()));
    }
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_example_line() {
        let line = "p@xx^xx-pau+r=a_xx%xx^00_00~00-1!1[xx$xx]xx/A:xx-xx-xx@xx~xx/B:1_1_1@xx|xx/C:2+1+1@JPN&0/D:xx!xx#xx$xx%xx|xx&xx;xx-xx/E:xx]xx^0=4/4~100!1@240#96+xx]1$1|0[24&0]96=0^100~xx#xx_xx;xx$xx&xx%xx[xx|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~xx+xx!xx^xx/F:C5#0#0-4/4$100$1+60%24;xx/G:xx_xx/H:xx_xx/I:8_8/J:2~2@1";

        let label = parse_label_line(line).expect("should parse");
        assert_eq!(label.phoneme.phoneme_id_current, "pau");
        assert_eq!(label.curr_note.reserved_3, "xx");
        assert_eq!(label.song.phrase_count, "1");
        assert_eq!(label.to_string(), line);

        insta::assert_debug_snapshot!(label);
    }

    #[test]
    fn parse_many_lines() {
        let lines = [
            "p@xx^xx-pau+r=a_xx%xx^00_00~00-1!1[xx$xx]xx/A:xx-xx-xx@xx~xx/B:1_1_1@xx|xx/C:2+1+1@JPN&0/D:xx!xx#xx$xx%xx|xx&xx;xx-xx/E:xx]xx^0=4/4~100!1@240#96+xx]1$1|0[24&0]96=0^100~xx#xx_xx;xx$xx&xx%xx[xx|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~xx+xx!xx^xx/F:C5#0#0-4/4$100$1+60%24;xx/G:xx_xx/H:xx_xx/I:8_8/J:2~2@1",
            "c@xx^pau-r+a=r_xx%00^00_00~00-1!2[xx$1]xx/A:xx-xx-xx@xx~xx/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:xx!xx#xx$xx%xx|xx&xx;xx-xx/E:C5]0^0=4/4~100!1@60#24+xx]1$4|0[24&0]96=0^100~1#8_0;48$0&192%0[100|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~xx+p2!xx^xx/F:D5#2#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@pau^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:xx-xx-xx@xx~xx/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:xx!xx#xx$xx%xx|xx&xx;xx-xx/E:C5]0^0=4/4~100!1@60#24+xx]1$4|0[24&0]96=0^100~1#8_0;48$0&192%0[100|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~xx+p2!xx^xx/F:D5#2#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=r_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:C5!0#0$4/4%100|1&60;24-xx/E:D5]2^0=4/4~100!1@60#24+xx]2$3|6[18&24]72=25^75~2#7_6;42$24&168%12[88|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p2!xx^xx/F:E5#4#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:C5!0#0$4/4%100|1&60;24-xx/E:D5]2^0=4/4~100!1@60#24+xx]2$3|6[18&24]72=25^75~2#7_6;42$24&168%12[88|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p2!xx^xx/F:E5#4#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=r_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:D5!2#0$4/4%100|1&60;24-xx/E:E5]4^0=4/4~100!1@60#24+xx]3$2|12[12&48]48=50^50~3#6_12;36$48&144%25[75|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p1!xx^xx/F:F5#5#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:D5!2#0$4/4%100|1&60;24-xx/E:E5]4^0=4/4~100!1@60#24+xx]3$2|12[12&48]48=50^50~3#6_12;36$48&144%25[75|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p1!xx^xx/F:F5#5#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=r_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:E5!4#0$4/4%100|1&60;24-xx/E:F5]5^0=4/4~100!1@60#24+xx]4$1|18[6&72]24=74^26~4#5_18;30$72&120%37[63|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m1+p2!xx^xx/F:G5#7#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:E5!4#0$4/4%100|1&60;24-xx/E:F5]5^0=4/4~100!1@60#24+xx]4$1|18[6&72]24=74^26~4#5_18;30$72&120%37[63|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m1+p2!xx^xx/F:G5#7#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=r_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:F5!5#0$4/4%100|1&60;24-xx/E:G5]7^0=4/4~100!1@60#24+xx]1$4|0[24&0]96=0^100~5#4_24;24$96&96%50[50|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p2!xx^xx/F:A5#9#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:F5!5#0$4/4%100|1&60;24-xx/E:G5]7^0=4/4~100!1@60#24+xx]1$4|0[24&0]96=0^100~5#4_24;24$96&96%50[50|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p2!xx^xx/F:A5#9#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=r_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:G5!7#0$4/4%100|1&60;24-xx/E:A5]9^0=4/4~100!1@60#24+xx]2$3|6[18&24]72=25^75~6#3_30;18$120&72%62[38|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p2!xx^xx/F:B5#11#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:G5!7#0$4/4%100|1&60;24-xx/E:A5]9^0=4/4~100!1@60#24+xx]2$3|6[18&24]72=25^75~6#3_30;18$120&72%62[38|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p2!xx^xx/F:B5#11#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=r_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:A5!9#0$4/4%100|1&60;24-xx/E:B5]11^0=4/4~100!1@60#24+xx]3$2|12[12&48]48=50^50~7#2_36;11$144&48%75[25|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p1!xx^xx/F:C6#0#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+r=a_00%00^00_00~00-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:2+1+1@JPN&0/D:A5!9#0$4/4%100|1&60;24-xx/E:B5]11^0=4/4~100!1@60#24+xx]3$2|12[12&48]48=50^50~7#2_36;11$144&48%75[25|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m2+p1!xx^xx/F:C6#0#0-4/4$100$1+60%24;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "c@r^a-r+a=pau_00%00^00_00~00-1!2[xx$1]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:xx+xx+xx@xx&xx/D:B5!11#0$4/4%100|1&60;24-xx/E:C6]0^0=4/4~100!1@60#24+xx]4$1|18[6&72]24=74^26~8#1_42;5$168&24%87[13|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m1+xx!xx^xx/F:xx#xx#xx-xx$xx$xx+xx%xx;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "v@a^r-a+pau=xx_00%00^00_00~xx-2!1[xx$xx]xx/A:2-1-1@JPN~0/B:2_1_1@JPN|0/C:xx+xx+xx@xx&xx/D:B5!11#0$4/4%100|1&60;24-xx/E:C6]0^0=4/4~100!1@60#24+xx]4$1|18[6&72]24=74^26~8#1_42;5$168&24%87[13|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~m1+xx!xx^xx/F:xx#xx#xx-xx$xx$xx+xx%xx;xx/G:xx_xx/H:8_8/I:xx_xx/J:2~2@1",
            "p@r^a-pau+xx=xx_00%00^00_xx~xx-1!1[xx$xx]xx/A:2-1-1@JPN~0/B:1_1_1@xx|xx/C:xx+xx+xx@xx&xx/D:C6!0#0$4/4%100|1&60;24-xx/E:xx]xx^0=4/4~100!1@240#96+xx]1$1|0[24&0]96=0^100~xx#xx_xx;xx$xx&xx%xx[xx|0]0-n^xx+xx~xx=xx@xx$xx!xx%xx#xx|xx|xx-xx&xx&xx+xx[xx;xx]xx;xx~xx~xx^xx^xx@xx[xx#xx=xx!xx~xx+xx!xx^xx/F:xx#xx#xx-xx$xx$xx+xx%xx;xx/G:8_8/H:xx_xx/I:xx_xx/J:2~2@1"
        ];
        let parsed_labels: Vec<Label> = lines
            .iter()
            .map(|line| parse_label_line(line).expect("should parse"))
            .collect();
        let reconstructed_lines: Vec<String> = parsed_labels
            .iter()
            .map(|label| label.to_string())
            .collect();
        assert_eq!(lines.len(), parsed_labels.len());
        assert_eq!(lines, reconstructed_lines.as_slice());

        insta::assert_debug_snapshot!(parsed_labels);
    }
}
