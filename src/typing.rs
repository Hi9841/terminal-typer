use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PracticeMode {
    Time,
    Words,
}

impl PracticeMode {
    pub const ALL: [Self; 2] = [Self::Time, Self::Words];

    pub fn label(self) -> &'static str {
        match self {
            Self::Time => "Time",
            Self::Words => "Words",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub finished_at: i64,
    pub mode: PracticeMode,
    pub amount: u32,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub errors: u32,
    pub elapsed_ms: u64,
    pub correct_chars: u32,
    #[serde(default)]
    pub wpm_samples: Vec<f64>,
    #[serde(default)]
    pub consistency: f64,
}

#[derive(Debug)]
pub struct TypingSession {
    pub target: Vec<char>,
    pub typed: Vec<char>,
    pub mode: PracticeMode,
    pub amount: u32,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub keystrokes: u32,
    pub correct_keystrokes: u32,
    pub errors: u32,
    /// When the last keypress landed, for the caret's idle blink.
    pub last_input_at: Option<Instant>,
    /// The cell the caret just left and when, for the caret's motion smear.
    pub caret_wake: Option<(usize, Instant)>,
    /// Where the last incorrect keystroke landed and when, for the error flash.
    pub last_error_at: Option<(usize, Instant)>,
    // (elapsed_ms, keystrokes, correct_keystrokes) snapshots, one per keypress.
    timeline: Vec<(u64, u32, u32)>,
}

/// Fills slots for characters the typist skipped past with a word commit so
/// they compare as incorrect without changing the rendered target text.
const SKIPPED: char = '\u{0}';

impl TypingSession {
    pub fn new(text: String, mode: PracticeMode, amount: u32) -> Self {
        Self {
            target: text.chars().collect(),
            typed: Vec::new(),
            mode,
            amount,
            started_at: None,
            finished_at: None,
            keystrokes: 0,
            correct_keystrokes: 0,
            errors: 0,
            last_input_at: None,
            caret_wake: None,
            last_error_at: None,
            timeline: Vec::new(),
        }
    }

    pub fn type_char(&mut self, value: char, now: Instant) {
        if self.is_finished() || self.typed.len() >= self.target.len() {
            return;
        }

        let started_at = *self.started_at.get_or_insert(now);
        if self.mode == PracticeMode::Time
            && now.duration_since(started_at) >= Duration::from_secs(self.amount as u64)
        {
            self.finish(now);
            return;
        }

        self.last_input_at = Some(now);
        let previous_caret = self.typed.len();

        let expected = self.target[self.typed.len()];
        if value == ' ' && expected != ' ' {
            // Space commits the current word: the untyped remainder is marked
            // wrong so the typist can move on instead of fixing every letter.
            let word_end = self.target[self.typed.len()..]
                .iter()
                .position(|c| *c == ' ')
                .map(|offset| self.typed.len() + offset)
                .unwrap_or(self.target.len());
            let skipped = word_end - self.typed.len();
            self.keystrokes += skipped as u32;
            self.errors += skipped as u32;
            for _ in 0..skipped {
                self.typed.push(SKIPPED);
            }
            if self.typed.len() >= self.target.len() {
                self.caret_wake = Some((previous_caret, now));
                return;
            }
        }

        let expected = self.target[self.typed.len()];
        self.keystrokes += 1;
        if value == expected {
            self.correct_keystrokes += 1;
        } else {
            self.errors += 1;
            self.last_error_at = Some((self.typed.len(), now));
        }
        self.typed.push(value);
        self.caret_wake = Some((previous_caret, now));
        self.timeline.push((
            now.duration_since(started_at).as_millis() as u64,
            self.keystrokes,
            self.correct_keystrokes,
        ));

        if self.mode == PracticeMode::Words && self.typed.len() == self.target.len() {
            self.finish(now);
        }
    }

    pub fn backspace(&mut self, now: Instant) {
        if self.is_finished() {
            return;
        }
        self.last_input_at = Some(now);
        let previous_caret = self.typed.len();
        // A run of skipped characters was never typed key by key, so deleting
        // into it removes the whole run, landing on the last real keystroke.
        if self.typed.pop().is_some() {
            while self.typed.last() == Some(&SKIPPED) {
                self.typed.pop();
            }
        }
        if self.typed.len() != previous_caret {
            self.caret_wake = Some((previous_caret, now));
        }
    }

    pub fn tick(&mut self, now: Instant) {
        let Some(started_at) = self.started_at else {
            return;
        };
        if self.mode == PracticeMode::Time
            && now.duration_since(started_at) >= Duration::from_secs(self.amount as u64)
        {
            self.finish(now);
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        let elapsed = match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => end.duration_since(start),
            (Some(start), None) => now.duration_since(start),
            _ => Duration::ZERO,
        };
        if self.mode == PracticeMode::Time {
            elapsed.min(Duration::from_secs(self.amount as u64))
        } else {
            elapsed
        }
    }

    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        if self.mode != PracticeMode::Time {
            return None;
        }
        let total = Duration::from_secs(self.amount as u64);
        Some(total.saturating_sub(self.elapsed(now)))
    }

    pub fn correct_chars(&self) -> usize {
        self.typed
            .iter()
            .zip(&self.target)
            .filter(|(typed, expected)| typed == expected)
            .count()
    }

    /// Characters credited to scored WPM. Timed tests credit correct partial
    /// words; word tests credit only words completed without an error.
    pub fn scored_correct_chars(&self) -> usize {
        if self.mode == PracticeMode::Time {
            return self.correct_chars();
        }

        let mut score = 0;
        let mut word_start = 0;
        for (index, expected) in self.target.iter().enumerate() {
            if *expected != ' ' {
                continue;
            }
            let word_end = index + 1;
            if self.typed.len() < word_end {
                break;
            }
            if self.typed[word_start..word_end] == self.target[word_start..word_end] {
                score += word_end - word_start;
            }
            word_start = word_end;
        }
        if word_start < self.target.len()
            && self.typed.len() >= self.target.len()
            && self.typed[word_start..] == self.target[word_start..]
        {
            score += self.target.len() - word_start;
        }
        score
    }

    pub fn accuracy(&self) -> f64 {
        if self.keystrokes == 0 {
            100.0
        } else {
            self.correct_keystrokes as f64 / self.keystrokes as f64 * 100.0
        }
    }

    pub fn record(&self) -> SessionRecord {
        let elapsed = self.elapsed(Instant::now());
        let accuracy = round_metric(self.accuracy());
        let wpm_samples = self.per_second_wpm();

        SessionRecord {
            finished_at: chrono::Utc::now().timestamp(),
            mode: self.mode,
            amount: self.amount,
            wpm: round_metric(calculate_wpm(self.scored_correct_chars(), elapsed)),
            raw_wpm: round_metric(calculate_wpm(self.keystrokes as usize, elapsed)),
            accuracy,
            errors: self.errors,
            elapsed_ms: elapsed.as_millis() as u64,
            correct_chars: self.correct_chars() as u32,
            consistency: round_metric(consistency(&wpm_samples)),
            wpm_samples,
        }
    }

    /// Correct WPM for each full second of the session, from keypress snapshots.
    fn per_second_wpm(&self) -> Vec<f64> {
        let seconds = self.elapsed(Instant::now()).as_secs();
        let correct_at = |boundary_ms: u64| {
            self.timeline
                .iter()
                .rev()
                .find(|(ms, _, _)| *ms < boundary_ms)
                .map(|entry| entry.2)
                .unwrap_or(0)
        };
        (1..=seconds)
            .map(|second| {
                let delta =
                    correct_at(second * 1000).saturating_sub(correct_at((second - 1) * 1000));
                delta as f64 * 12.0
            })
            .collect()
    }

    fn finish(&mut self, now: Instant) {
        if self.finished_at.is_none() {
            self.finished_at = Some(if self.mode == PracticeMode::Time {
                self.started_at
                    .map(|start| start + Duration::from_secs(self.amount as u64))
                    .unwrap_or(now)
            } else {
                now
            });
        }
    }
}

fn calculate_wpm(characters: usize, elapsed: Duration) -> f64 {
    let minutes = elapsed.as_secs_f64() / 60.0;
    if minutes <= 0.0 {
        0.0
    } else {
        characters as f64 / 5.0 / minutes
    }
}

/// 100 minus the coefficient of variation of the per-second samples, clamped.
fn consistency(samples: &[f64]) -> f64 {
    if samples.len() < 2 {
        return 100.0;
    }
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    if mean <= f64::EPSILON {
        return 0.0;
    }
    let variance = samples
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    (100.0 * (1.0 - variance.sqrt() / mean)).clamp(0.0, 100.0)
}

fn round_metric(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_corrected_errors_without_leaving_current_error() {
        let start = Instant::now();
        let mut session = TypingSession::new("cat ".into(), PracticeMode::Words, 1);

        session.type_char('c', start);
        session.type_char('x', start + Duration::from_secs(1));
        session.backspace(start + Duration::from_secs(1));
        session.type_char('a', start + Duration::from_secs(2));
        session.type_char('t', start + Duration::from_secs(3));
        session.type_char(' ', start + Duration::from_secs(4));

        assert!(session.is_finished());
        assert_eq!(session.errors, 1);
        assert_eq!(session.keystrokes, 5);
        assert_eq!(session.correct_keystrokes, 4);
    }

    #[test]
    fn word_session_finishes_with_an_error_after_commit_space() {
        let start = Instant::now();
        let mut session = TypingSession::new("cat ".into(), PracticeMode::Words, 1);

        for (offset, value) in ['c', 'a', 'x', ' '].into_iter().enumerate() {
            session.type_char(value, start + Duration::from_secs(offset as u64 + 1));
        }

        assert!(session.is_finished());
        assert_eq!(session.errors, 1);
        assert_eq!(session.scored_correct_chars(), 0);
    }

    #[test]
    fn timed_session_finishes_at_deadline() {
        let start = Instant::now();
        let mut session = TypingSession::new("hello world".into(), PracticeMode::Time, 15);
        session.type_char('h', start);
        session.tick(start + Duration::from_secs(14));
        assert!(!session.is_finished());

        session.tick(start + Duration::from_secs(15));
        assert!(session.is_finished());
        assert_eq!(
            session.elapsed(start + Duration::from_secs(30)),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn space_commits_incomplete_word_and_marks_remainder_wrong() {
        let start = Instant::now();
        let mut session = TypingSession::new("cat dog ".into(), PracticeMode::Words, 2);

        session.type_char('c', start);
        session.type_char(' ', start + Duration::from_secs(1));

        assert!(!session.is_finished());
        assert_eq!(session.typed, vec!['c', SKIPPED, SKIPPED, ' ']);
        assert_eq!(session.errors, 2);

        for (offset, value) in ['d', 'o', 'g', ' '].into_iter().enumerate() {
            session.type_char(value, start + Duration::from_secs(offset as u64 + 2));
        }

        assert!(session.is_finished());
        assert_eq!(session.errors, 2);
        // Only the second word scores: "dog " is 4 of the 8 target characters.
        assert_eq!(session.scored_correct_chars(), 4);
    }

    #[test]
    fn backspace_can_return_into_a_committed_word() {
        let start = Instant::now();
        let mut session = TypingSession::new("cat dog ".into(), PracticeMode::Words, 2);

        session.type_char('c', start);
        session.type_char(' ', start + Duration::from_secs(1));
        session.backspace(start + Duration::from_secs(2));

        assert_eq!(session.typed, vec!['c']);
    }

    #[test]
    fn record_includes_per_second_wpm_samples() {
        let start = Instant::now();
        let mut session = TypingSession::new("aaaa bbbb ".into(), PracticeMode::Words, 2);

        for index in 0..5u64 {
            session.type_char(
                session.target[index as usize],
                start + Duration::from_millis(100 + index * 200),
            );
        }
        for index in 5..10u64 {
            session.type_char(
                session.target[index as usize],
                start + Duration::from_millis(1100 + (index - 5) * 200),
            );
        }

        assert!(session.is_finished());
        let record = session.record();
        assert_eq!(record.wpm_samples, vec![60.0]);
        assert_eq!(record.consistency, 100.0);
    }

    #[test]
    fn wpm_uses_standard_five_character_word() {
        assert_eq!(calculate_wpm(250, Duration::from_secs(60)), 50.0);
    }
}
