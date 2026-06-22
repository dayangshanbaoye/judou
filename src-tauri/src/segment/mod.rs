use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentationMethod {
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SegmentedSentence {
    pub text: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub method: SegmentationMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SegmentationNotice {
    pub rule_name: String,
    pub offset: usize,
    pub snippet: String,
    pub action_taken: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SegmentationOutput {
    pub sentences: Vec<SegmentedSentence>,
    pub notices: Vec<SegmentationNotice>,
}

pub fn segment_paragraph(paragraph: &str) -> Vec<SegmentedSentence> {
    segment_paragraph_with_notices(paragraph).sentences
}

pub fn segment_paragraph_with_notices(paragraph: &str) -> SegmentationOutput {
    if paragraph.is_empty() {
        return SegmentationOutput {
            sentences: Vec::new(),
            notices: Vec::new(),
        };
    }

    let mut sentences = Vec::new();
    let mut notices = Vec::new();
    let mut sentence_start = 0usize;
    let mut index = 0usize;

    while index < paragraph.len() {
        let rest = match paragraph.get(index..) {
            Some(rest) => rest,
            None => break,
        };
        let ch = match rest.chars().next() {
            Some(ch) => ch,
            None => break,
        };
        let next_index = index + ch.len_utf8();

        if let Some(notice) = protected_period_notice(paragraph, index, ch) {
            notices.push(notice);
            index = next_index;
        } else if is_sentence_terminator(ch) {
            let end = consume_following_whitespace(paragraph, next_index);
            push_sentence(paragraph, sentence_start, end, &mut sentences);
            sentence_start = end;
            index = end;
        } else {
            index = next_index;
        }
    }

    if sentence_start < paragraph.len() {
        push_sentence(paragraph, sentence_start, paragraph.len(), &mut sentences);
    }

    SegmentationOutput { sentences, notices }
}

fn push_sentence(
    paragraph: &str,
    start: usize,
    end: usize,
    sentences: &mut Vec<SegmentedSentence>,
) {
    if start >= end {
        return;
    }

    if let Some(text) = paragraph.get(start..end) {
        sentences.push(SegmentedSentence {
            text: text.to_string(),
            start_offset: start,
            end_offset: end,
            method: SegmentationMethod::Rule,
        });
    }
}

fn is_sentence_terminator(ch: char) -> bool {
    matches!(ch, '.' | '!' | '?')
}

fn protected_period_notice(
    paragraph: &str,
    period_index: usize,
    ch: char,
) -> Option<SegmentationNotice> {
    if ch != '.' {
        return None;
    }

    if is_decimal_period(paragraph, period_index) {
        return Some(notice(paragraph, period_index, "decimal", "kept decimal point inside sentence"));
    }
    if is_known_abbreviation_period(paragraph, period_index)
        || is_multi_initial_abbreviation_period(paragraph, period_index)
        || is_initial_period(paragraph, period_index)
    {
        return Some(notice(
            paragraph,
            period_index,
            "abbreviation",
            "kept abbreviation period inside sentence",
        ));
    }
    None
}

fn notice(paragraph: &str, offset: usize, rule_name: &str, action_taken: &str) -> SegmentationNotice {
    let start = offset.saturating_sub(12);
    let end = (offset + 12).min(paragraph.len());
    let snippet = match paragraph.get(start..end) {
        Some(snippet) => snippet.to_string(),
        None => String::new(),
    };
    SegmentationNotice {
        rule_name: rule_name.to_string(),
        offset,
        snippet,
        action_taken: action_taken.to_string(),
    }
}

fn is_decimal_period(paragraph: &str, period_index: usize) -> bool {
    previous_char(paragraph, period_index).is_some_and(|ch| ch.is_ascii_digit())
        && next_char(paragraph, period_index).is_some_and(|ch| ch.is_ascii_digit())
}

fn is_known_abbreviation_period(paragraph: &str, period_index: usize) -> bool {
    let prefix = match paragraph.get(..=period_index) {
        Some(prefix) => prefix.to_ascii_lowercase(),
        None => return false,
    };

    known_abbreviations()
        .iter()
        .any(|abbreviation| prefix.ends_with(abbreviation))
}

fn is_multi_initial_abbreviation_period(paragraph: &str, period_index: usize) -> bool {
    let next = match next_char(paragraph, period_index) {
        Some(next) => next,
        None => return false,
    };
    if !next.is_ascii_uppercase() {
        return false;
    }

    let next_index = period_index + 1 + next.len_utf8();
    matches!(paragraph.as_bytes().get(next_index), Some(b'.'))
}

fn known_abbreviations() -> &'static [&'static str] {
    &[
        "mr.", "mrs.", "ms.", "dr.", "prof.", "sr.", "jr.", "st.", "e.g.", "i.e.", "etc.",
        "u.s.",
    ]
}

fn is_initial_period(paragraph: &str, period_index: usize) -> bool {
    let previous = match previous_char(paragraph, period_index) {
        Some(previous) => previous,
        None => return false,
    };

    if !previous.is_ascii_uppercase() {
        return false;
    }

    let before_previous = previous_char(paragraph, period_index.saturating_sub(previous.len_utf8()));
    let next = next_char(paragraph, period_index);

    let has_initial_left_boundary = before_previous.is_none_or(|ch| ch.is_whitespace());
    let has_initial_right_boundary = next.is_none_or(|ch| ch.is_whitespace());

    has_initial_left_boundary && has_initial_right_boundary
}

fn consume_following_whitespace(paragraph: &str, start: usize) -> usize {
    let mut end = start;
    while end < paragraph.len() {
        let rest = match paragraph.get(end..) {
            Some(rest) => rest,
            None => break,
        };
        let ch = match rest.chars().next() {
            Some(ch) => ch,
            None => break,
        };
        if !ch.is_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    end
}

fn previous_char(paragraph: &str, index: usize) -> Option<char> {
    paragraph.get(..index)?.chars().next_back()
}

fn next_char(paragraph: &str, index: usize) -> Option<char> {
    let after_period = index.checked_add(1)?;
    paragraph.get(after_period..)?.chars().next()
}
