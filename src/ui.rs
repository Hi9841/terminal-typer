use std::time::{Duration, Instant};

use chrono::{Local, TimeZone};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Row, Table},
};

use crate::{
    app::{App, Screen, TIME_OPTIONS, WORD_OPTIONS},
    typing::{PracticeMode, SessionRecord, TypingSession},
    words,
};

// A small, deliberate palette: cool paper text, mint accent, coral errors.
// Colors only style foreground; the terminal's native background (including
// transparency) is left untouched.
const SUB_ALT: Color = Color::Rgb(31, 38, 42);
const TEXT: Color = Color::Rgb(228, 236, 234);
const MUTED: Color = Color::Rgb(121, 137, 143);
const ACCENT: Color = Color::Rgb(126, 226, 178);
const RED: Color = Color::Rgb(240, 107, 105);

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if area.width < 58 || area.height < 22 {
        draw_too_small(frame, area);
        return;
    }

    match app.screen {
        Screen::Menu => draw_menu(frame, app, area),
        Screen::Practice => draw_practice(frame, app, area),
        Screen::Results => draw_results(frame, app, area),
        Screen::History => draw_history(frame, app, area),
    }
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let copy = Paragraph::new(vec![
        Line::styled("terminal typer", Style::default().fg(ACCENT).bold()),
        Line::raw(""),
        Line::styled("Resize the terminal to continue", Style::default().fg(TEXT)),
        Line::styled("minimum: 58 x 22", Style::default().fg(MUTED)),
    ])
    .alignment(Alignment::Center);
    frame.render_widget(copy, centered(area, 44, 6));
}

fn draw_menu(frame: &mut Frame, app: &App, area: Rect) {
    let root = content_area(area);
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Min(4),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Length(if app.status.is_some() { 1 } else { 0 }),
    ])
    .split(root);

    draw_topbar(frame, sections[0], "practice");

    draw_settings(frame, app, sections[1]);

    let actions = centered(sections[2], 34, 3);
    let action_rows = Layout::vertical([Constraint::Length(1); 3]).split(actions);
    action_line(
        frame,
        action_rows[0],
        "start practice",
        app.menu_focus == 5,
        ACCENT,
    );
    action_line(frame, action_rows[1], "history", app.menu_focus == 6, TEXT);
    action_line(frame, action_rows[2], "quit", app.menu_focus == 7, MUTED);

    let best = app.history.iter().map(|item| item.wpm).fold(0.0, f64::max);
    let average_accuracy = if app.history.is_empty() {
        None
    } else {
        Some(app.history.iter().map(|item| item.accuracy).sum::<f64>() / app.history.len() as f64)
    };
    let recent_average = if app.history.is_empty() {
        None
    } else {
        let recent = app.history.len().saturating_sub(10);
        let slice = &app.history[recent..];
        Some(slice.iter().map(|item| item.wpm).sum::<f64>() / slice.len() as f64)
    };
    let stats = format!(
        "sessions {:>3}   best {:>3.0} wpm   last 10 {} wpm   avg accuracy {}",
        app.history.len(),
        best,
        recent_average
            .map(|value| format!("{value:>3.0}"))
            .unwrap_or_else(|| " -".into()),
        average_accuracy
            .map(|value| format!("{value:>5.1}%"))
            .unwrap_or_else(|| "    -".into())
    );
    frame.render_widget(
        Paragraph::new(stats)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center),
        sections[4],
    );

    if let Some(status) = &app.status {
        frame.render_widget(
            Paragraph::new(status.as_str()).style(Style::default().fg(RED)),
            sections[5],
        );
    }
}

fn draw_topbar(frame: &mut Frame, area: Rect, section: &str) {
    let chunks = Layout::horizontal([Constraint::Min(20), Constraint::Length(18)]).split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("terminal", Style::default().fg(TEXT).bold()),
            Span::styled(" typer", Style::default().fg(ACCENT).bold()),
        ])),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(section)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Right),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("-".repeat(area.width as usize)).style(Style::default().fg(SUB_ALT)),
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: 1,
        },
    );
}

fn draw_settings(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::vertical([Constraint::Length(1); 5]).split(area);
    let mode_values = PracticeMode::ALL
        .iter()
        .map(|mode| mode.label())
        .collect::<Vec<_>>();
    setting_line(
        frame,
        rows[0],
        "mode",
        &mode_values,
        app.mode_index,
        app.menu_focus == 0,
    );

    let (values, selected): (Vec<String>, usize) = match app.selected_mode() {
        PracticeMode::Time => (
            TIME_OPTIONS
                .iter()
                .map(|value| format!("{value}s"))
                .collect(),
            app.time_index,
        ),
        PracticeMode::Words => (
            WORD_OPTIONS.iter().map(ToString::to_string).collect(),
            app.word_index,
        ),
    };
    let value_refs = values.iter().map(String::as_str).collect::<Vec<_>>();
    setting_line(
        frame,
        rows[1],
        "length",
        &value_refs,
        selected,
        app.menu_focus == 1,
    );
    setting_line(
        frame,
        rows[2],
        "punctuation",
        &["off", "on"],
        usize::from(app.punctuation),
        app.menu_focus == 2,
    );
    setting_line(
        frame,
        rows[3],
        "numbers",
        &["off", "on"],
        usize::from(app.numbers),
        app.menu_focus == 3,
    );
    setting_line(
        frame,
        rows[4],
        "list",
        &words::SET_LABELS,
        usize::from(app.extended_words),
        app.menu_focus == 4,
    );
}

fn setting_line(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    values: &[&str],
    selected: usize,
    focused: bool,
) {
    let mut spans = vec![Span::styled(
        if focused {
            format!("> {label:<12}")
        } else {
            format!("  {label:<12}")
        },
        if focused {
            Style::default().fg(ACCENT).bold()
        } else {
            Style::default().fg(MUTED)
        },
    )];
    for (index, value) in values.iter().enumerate() {
        spans.push(Span::raw(if index == 0 { "" } else { "  " }));
        spans.push(Span::styled(
            *value,
            if index == selected {
                Style::default()
                    .fg(if focused { ACCENT } else { TEXT })
                    .bold()
            } else {
                Style::default().fg(MUTED)
            },
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn action_line(frame: &mut Frame, area: Rect, label: &str, focused: bool, color: Color) {
    let marker = if focused { "> " } else { "  " };
    let style = if focused {
        Style::default().fg(color).bold()
    } else {
        Style::default().fg(TEXT)
    };
    frame.render_widget(
        Paragraph::new(format!("{marker}{label}")).style(style),
        area,
    );
}

fn draw_practice(frame: &mut Frame, app: &App, area: Rect) {
    let Some(session) = &app.session else {
        return;
    };
    let root = content_area(area);
    // Monkeytype purity during a test: the counter and the words, nothing else.
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Fill(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Fill(3),
    ])
    .split(root);

    draw_topbar(frame, sections[0], "");

    let now = Instant::now();
    let (primary, alignment) = match session.remaining(now) {
        Some(remaining) => (
            format!("{:.0}s", remaining.as_secs_f64().ceil()),
            Alignment::Center,
        ),
        None => (
            format!("{}/{}", typed_words(session), session.amount),
            Alignment::Left,
        ),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(primary, Style::default().fg(ACCENT).bold()))
            .alignment(alignment),
        sections[2],
    );

    draw_prompt(frame, session, sections[4]);
}

/// A three-line window onto the passage: the active line stays in the middle
/// and lines scroll away upward as the typist advances, Monkeytype style.
fn draw_prompt(frame: &mut Frame, session: &TypingSession, area: Rect) {
    let width = area.width.max(1) as usize;
    let (lines, cursor_line) = prompt_lines(session, width);
    let visible = area.height as usize;
    let max_scroll = lines.len().saturating_sub(visible);
    let scroll = cursor_line.saturating_sub(visible / 2).min(max_scroll) as u16;
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .scroll((scroll, 0)),
        area,
    );
}

/// Block caret: the next expected character is filled with the accent color,
/// so the caret always sits exactly on the letter to type and never shifts
/// the surrounding text.
const CARET_FG: Color = Color::Rgb(17, 20, 23);

// Motion timing, in the spirit of Monkeytype: near-invisible polish while
// typing (smear, flash, blink), personality on the results reveal.
const CARET_SMEAR_MS: u64 = 70;
const CARET_WAKE_MS: u64 = 140;
const ERROR_FLASH_MS: u64 = 150;
const CARET_BLINK_DELAY_MS: u64 = 500;
const CARET_BLINK_PERIOD_MS: u64 = 530;
const WPM_COUNT_UP_MS: u64 = 600;
const RESULTS_DETAILS_MS: u64 = 200;
const RESULTS_GRAPH_MS: u64 = 150;
const RESULTS_GRAPH_GROW_MS: u64 = 250;
const RESULTS_GRAPH_LABEL_MS: u64 = 350;
const RESULTS_PB_MS: u64 = 650;

fn caret_style() -> Style {
    Style::default().fg(CARET_FG).bg(ACCENT)
}

fn ease_out_cubic(progress: f64) -> f64 {
    1.0 - (1.0 - progress).powi(3)
}

/// Solid while typing; blinks after an idle pause, like a real caret.
fn caret_visible(last_input_at: Option<Instant>, now: Instant) -> bool {
    let Some(last_input) = last_input_at else {
        return true;
    };
    let idle = now.duration_since(last_input).as_millis() as u64;
    if idle < CARET_BLINK_DELAY_MS {
        true
    } else {
        ((idle - CARET_BLINK_DELAY_MS) / CARET_BLINK_PERIOD_MS).is_multiple_of(2)
    }
}

/// Style for one character cell of the passage, including animation states.
fn cell_style(
    session: &TypingSession,
    position: usize,
    now: Instant,
    caret_visible: bool,
) -> Style {
    // The cell the caret just left: a full smear that fades through accent.
    if let Some((wake_position, at)) = session.caret_wake
        && position == wake_position
        && position < session.typed.len()
    {
        let age = now.duration_since(at).as_millis() as u64;
        if age < CARET_SMEAR_MS {
            return caret_style();
        }
        if age < CARET_WAKE_MS {
            return Style::default().fg(ACCENT);
        }
    }

    if position < session.typed.len() {
        let correct = session.typed[position] == session.target[position];
        let mut style = if correct {
            Style::default().fg(TEXT)
        } else {
            Style::default().fg(RED).add_modifier(Modifier::UNDERLINED)
        };
        // A fresh mistake lands bold for a beat before settling.
        let flash_age = session
            .last_error_at
            .filter(|(error_position, at)| {
                *error_position == position
                    && (now.duration_since(*at).as_millis() as u64) < ERROR_FLASH_MS
            })
            .is_some();
        if !correct && flash_age {
            style = style.add_modifier(Modifier::BOLD);
        }
        style
    } else if position == session.typed.len() && caret_visible {
        caret_style()
    } else {
        Style::default().fg(MUTED)
    }
}

/// Wraps the passage into lines of (char, style), then justifies every line
/// except the last so the right edge of the text block is straight instead
/// of ragged. Justification is stable while typing: line contents never
/// change after wrapping, only styles do.
fn prompt_lines(session: &TypingSession, width: usize) -> (Vec<Line<'static>>, usize) {
    let now = Instant::now();
    let visible = caret_visible(session.last_input_at, now);
    let mut lines: Vec<Vec<(char, Style)>> = Vec::new();
    let mut current: Vec<(char, Style)> = Vec::new();
    let mut cursor_line = 0usize;
    let mut index = 0usize;

    while index < session.target.len() {
        let mut end = index;
        while end < session.target.len() && session.target[end] != ' ' {
            end += 1;
        }
        if end < session.target.len() {
            end += 1;
        }
        let chunk_width = end - index;
        if !current.is_empty() && current.len() + chunk_width > width {
            lines.push(std::mem::take(&mut current));
        }
        if session.typed.len() >= index && session.typed.len() < end {
            cursor_line = lines.len();
        }
        for position in index..end {
            let expected = session.target[position];
            current.push((expected, cell_style(session, position, now, visible)));
        }
        index = end;
    }
    if !current.is_empty() {
        lines.push(current);
    }

    let last_line = lines.len().saturating_sub(1);
    let rendered = lines
        .into_iter()
        .enumerate()
        .map(|(line_index, line)| {
            let cells = if line_index == last_line {
                line
            } else {
                justify_line(line, width)
            };
            to_line(cells)
        })
        .collect();
    (rendered, cursor_line)
}

/// Stretches one wrapped line to the full width by widening the word gaps.
fn justify_line(line: Vec<(char, Style)>, width: usize) -> Vec<(char, Style)> {
    let gaps = line.iter().filter(|(value, _)| *value == ' ').count();
    let extra = width.saturating_sub(line.len());
    if gaps == 0 || extra == 0 {
        return line;
    }
    let per_gap = extra / gaps;
    let mut remainder = extra % gaps;
    let mut justified = Vec::with_capacity(width);
    for (value, style) in line {
        justified.push((value, style));
        if value == ' ' {
            let pad = per_gap + usize::from(remainder > 0);
            remainder = remainder.saturating_sub(1);
            for _ in 0..pad {
                justified.push((' ', Style::default().fg(MUTED)));
            }
        }
    }
    justified
}

/// Merges consecutive equally-styled characters into spans.
fn to_line(cells: Vec<(char, Style)>) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    for (value, style) in cells {
        match spans.last_mut() {
            Some(last) if last.style == style => last.content.to_mut().push(value),
            _ => spans.push(Span::styled(value.to_string(), style)),
        }
    }
    Line::from(spans)
}

fn draw_results(frame: &mut Frame, app: &App, area: Rect) {
    let Some(result) = &app.last_result else {
        return;
    };
    let root = content_area(area);
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Min(6),
        Constraint::Length(if app.status.is_some() { 1 } else { 0 }),
    ])
    .split(root);
    draw_topbar(frame, sections[0], "result");

    // Reveal timeline: the WPM count-up runs first, then supporting details,
    // the growing chart, and finally the personal-best flourish.
    let shown = app
        .result_shown_at
        .map(|at| Instant::now().duration_since(at))
        .unwrap_or(Duration::from_secs(3600));
    let shown_ms = shown.as_millis() as u64;
    let count_up = ease_out_cubic(
        (shown_ms.min(WPM_COUNT_UP_MS) as f64 / WPM_COUNT_UP_MS as f64).clamp(0.0, 1.0),
    );

    let mut primary = vec![
        Line::from(vec![
            Span::styled(
                format!("{:.0}", result.wpm * count_up),
                Style::default().fg(ACCENT).bold(),
            ),
            Span::styled(" wpm", Style::default().fg(MUTED)),
        ]),
        Line::styled(record_mode(result), Style::default().fg(MUTED)),
    ];
    if app.new_best && shown_ms >= RESULTS_PB_MS {
        primary.push(Line::styled(
            "new personal best",
            Style::default().fg(ACCENT).bold(),
        ));
    }
    frame.render_widget(
        Paragraph::new(primary).alignment(Alignment::Center),
        sections[1],
    );
    if shown_ms >= RESULTS_DETAILS_MS {
        let details = format!(
            "raw {:>3.0}    accuracy {:>5.1}%    consistency {:>5.1}%    errors {:>2}",
            result.raw_wpm, result.accuracy, result.consistency, result.errors
        );
        frame.render_widget(
            Paragraph::new(details)
                .style(Style::default().fg(TEXT))
                .alignment(Alignment::Center),
            sections[2],
        );
    }

    draw_wpm_graph(frame, sections[3], &result.wpm_samples, shown);

    let actions = centered(sections[4], 34, 4);
    let action_rows = Layout::vertical([Constraint::Length(1); 4]).split(actions);
    action_line(
        frame,
        action_rows[0],
        "retry",
        app.results_focus == 0,
        ACCENT,
    );
    action_line(
        frame,
        action_rows[1],
        "practice menu",
        app.results_focus == 1,
        TEXT,
    );
    action_line(
        frame,
        action_rows[2],
        "history",
        app.results_focus == 2,
        TEXT,
    );
    action_line(frame, action_rows[3], "quit", app.results_focus == 3, MUTED);

    if let Some(status) = &app.status {
        frame.render_widget(
            Paragraph::new(status.as_str()).style(Style::default().fg(RED)),
            sections[5],
        );
    }
}

/// A quiet one-line bar chart of per-second WPM, scaled to the available
/// width. Columns grow in a quick left-to-right stagger after the reveal.
fn draw_wpm_graph(frame: &mut Frame, area: Rect, samples: &[f64], shown: Duration) {
    if samples.is_empty() || area.width < 20 {
        return;
    }
    let shown_ms = shown.as_millis() as u64;
    let width = area.width.saturating_sub(2) as usize;
    let columns = if samples.len() > width {
        (0..width)
            .map(|column| {
                let start = column * samples.len() / width;
                let end = ((column + 1) * samples.len() / width).max(start + 1);
                let slice = &samples[start..end.min(samples.len())];
                slice.iter().sum::<f64>() / slice.len() as f64
            })
            .collect::<Vec<_>>()
    } else {
        samples.to_vec()
    };
    let peak = samples.iter().copied().fold(0.0_f64, f64::max);

    let blocks = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    // Stagger so the wave crosses the chart in at most ~300ms.
    let stagger = (12u64).min((300 / columns.len() as u64).max(1));
    let bars: String = columns
        .iter()
        .enumerate()
        .map(|(column, value)| {
            let level = if peak <= f64::EPSILON {
                0.0
            } else {
                (value / peak) * (blocks.len() - 1) as f64
            };
            let start = RESULTS_GRAPH_MS + column as u64 * stagger;
            let progress = shown_ms.saturating_sub(start).min(RESULTS_GRAPH_GROW_MS) as f64
                / RESULTS_GRAPH_GROW_MS as f64;
            let level = (level * ease_out_cubic(progress)).round() as usize;
            blocks[level.min(blocks.len() - 1)]
        })
        .collect();

    let rows = Layout::vertical([Constraint::Length(1); 3]).split(area);
    let label = if shown_ms >= RESULTS_GRAPH_LABEL_MS {
        "wpm by second"
    } else {
        ""
    };
    frame.render_widget(
        Paragraph::new(label)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(bars)
            .style(Style::default().fg(ACCENT))
            .alignment(Alignment::Center),
        rows[1],
    );
    let peak_label = if shown_ms >= RESULTS_GRAPH_LABEL_MS {
        format!("peak {:>3.0} wpm", peak)
    } else {
        String::new()
    };
    frame.render_widget(
        Paragraph::new(peak_label)
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center),
        rows[2],
    );
}

fn draw_history(frame: &mut Frame, app: &App, area: Rect) {
    let root = content_area(area);
    let sections = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(8),
        Constraint::Length(2),
    ])
    .split(root);
    draw_topbar(frame, sections[0], "history");

    if app.history.is_empty() {
        frame.render_widget(
            Paragraph::new("No completed sessions yet.")
                .style(Style::default().fg(MUTED))
                .alignment(Alignment::Center),
            sections[1],
        );
    } else {
        let header = Row::new(["when", "mode", "wpm", "raw", "accuracy", "errors"])
            .style(Style::default().fg(MUTED).add_modifier(Modifier::BOLD));
        let available = sections[1].height.saturating_sub(2) as usize;
        let rows = app.history.iter().rev().take(available).map(|item| {
            let when = Local
                .timestamp_opt(item.finished_at, 0)
                .single()
                .map(|value| value.format("%b %d %H:%M").to_string())
                .unwrap_or_else(|| "unknown".into());
            Row::new([
                when,
                record_mode(item),
                format!("{:>3.0}", item.wpm),
                format!("{:>3.0}", item.raw_wpm),
                format!("{:>5.1}%", item.accuracy),
                format!("{:>2}", item.errors),
            ])
            .style(Style::default().fg(TEXT))
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Length(12),
                Constraint::Length(6),
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Length(6),
            ],
        )
        .header(header)
        .column_spacing(2);
        frame.render_widget(table, sections[1]);
    }
    frame.render_widget(
        Paragraph::new("< back")
            .style(Style::default().fg(MUTED))
            .alignment(Alignment::Center),
        sections[2],
    );
}

fn content_area(area: Rect) -> Rect {
    area.inner(Margin {
        horizontal: if area.width >= 100 { 8 } else { 3 },
        vertical: 1,
    })
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let horizontal = Layout::new(
        Direction::Horizontal,
        [
            Constraint::Fill(1),
            Constraint::Length(width.min(area.width)),
            Constraint::Fill(1),
        ],
    )
    .split(area);
    Layout::new(
        Direction::Vertical,
        [
            Constraint::Fill(1),
            Constraint::Length(height.min(area.height)),
            Constraint::Fill(1),
        ],
    )
    .split(horizontal[1])[1]
}

fn record_mode(record: &SessionRecord) -> String {
    match record.mode {
        PracticeMode::Time => format!("{} seconds", record.amount),
        PracticeMode::Words => format!("{} words", record.amount),
    }
}

fn typed_words(session: &TypingSession) -> u32 {
    let completed_spaces = session
        .typed
        .iter()
        .zip(&session.target)
        .filter(|(typed, expected)| **typed == ' ' && typed == expected)
        .count() as u32;
    if session.typed.is_empty() {
        0
    } else {
        completed_spaces + 1
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::typing::{PracticeMode, TypingSession};

    #[test]
    fn menu_renders_at_standard_terminal_size() {
        let app = App::new().expect("test app should initialize");
        let output = render(&app, 80, 24);

        assert!(output.contains("terminal typer"));
        assert!(output.contains("start practice"));
        assert!(output.contains("mode"));
        assert!(output.contains("punctuation"));
        assert!(output.contains("numbers"));
    }

    #[test]
    fn results_screen_shows_graph_and_personal_best() {
        let mut app = App::new().expect("test app should initialize");
        app.screen = Screen::Results;
        app.new_best = true;
        app.last_result = Some(SessionRecord {
            finished_at: 1_800_000_000,
            mode: PracticeMode::Words,
            amount: 25,
            wpm: 72.0,
            raw_wpm: 75.0,
            accuracy: 96.0,
            errors: 2,
            elapsed_ms: 20_000,
            correct_chars: 140,
            wpm_samples: vec![50.0, 60.0, 55.0],
            consistency: 90.0,
        });
        let output = render(&app, 80, 24);

        assert!(output.contains("72 wpm"));
        assert!(output.contains("new personal best"));
        assert!(output.contains("consistency"));
        assert!(output.contains("wpm by second"));
        assert!(output.contains('█'));
    }

    #[test]
    fn practice_renders_at_standard_terminal_size() {
        let mut app = App::new().expect("test app should initialize");
        app.screen = Screen::Practice;
        app.session = Some(TypingSession::new(
            "practice makes progress ".into(),
            PracticeMode::Words,
            3,
        ));
        let output = render(&app, 80, 24);

        assert!(output.contains("0/3"));
        assert!(output.contains("practice"));
        assert!(output.contains("makes"));
    }

    #[test]
    fn practice_shows_three_line_scrolling_window() {
        let mut app = App::new().expect("test app should initialize");
        app.screen = Screen::Practice;
        // 40 five-letter words wrap to four lines at an 80-column terminal.
        let words: Vec<String> = std::iter::once("first".to_string())
            .chain((0..38).map(|_| "wwwww".to_string()))
            .chain(std::iter::once("final".to_string()))
            .collect();
        let mut session =
            TypingSession::new(format!("{} ", words.join(" ")), PracticeMode::Words, 40);
        let target = session.target.clone();
        // Type past the third wrapped line so the first line must scroll out.
        for expected in target.iter().take(150) {
            session.type_char(*expected, Instant::now());
        }
        app.session = Some(session);
        let output = render(&app, 80, 24);

        assert!(!output.contains("first"));
        assert!(output.contains("final"));
    }

    #[test]
    fn compact_terminal_uses_fallback_view() {
        let app = App::new().expect("test app should initialize");
        let output = render(&app, 57, 21);

        assert!(output.contains("Resize the terminal"));
        assert!(!output.contains("start practice"));
    }

    #[test]
    fn caret_fills_the_next_expected_character() {
        let mut app = App::new().expect("test app should initialize");
        app.screen = Screen::Practice;
        let mut session = TypingSession::new("hello world ".into(), PracticeMode::Words, 2);
        session.type_char('h', Instant::now());
        app.session = Some(session);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        terminal
            .draw(|frame| draw(frame, &app))
            .expect("frame should render");

        // Right after a keystroke the caret block sits on the next character
        // and the cell just left still carries the motion smear.
        let highlighted: Vec<&str> = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .filter(|cell| cell.bg == ACCENT)
            .map(|cell| cell.symbol())
            .collect();
        assert_eq!(highlighted, ["h", "e"]);
    }

    #[test]
    fn wrapped_lines_are_justified_to_equal_width() {
        let session = TypingSession::new("aa bb cc dd ee ff gg hh ".into(), PracticeMode::Words, 8);
        let (lines, _) = prompt_lines(&session, 10);

        assert!(lines.len() >= 2, "passage should wrap at width 10");
        for line in &lines[..lines.len() - 1] {
            let width: usize = line
                .spans
                .iter()
                .map(|span| span.content.chars().count())
                .sum();
            assert_eq!(width, 10, "every wrapped line fills the full width");
        }
    }

    fn render(app: &App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        terminal
            .draw(|frame| draw(frame, app))
            .expect("frame should render");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }
}
