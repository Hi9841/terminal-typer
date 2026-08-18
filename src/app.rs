use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;

use crate::{
    storage::{Settings, Storage},
    typing::{PracticeMode, SessionRecord, TypingSession},
    ui, words,
};

pub const MENU_ROWS: usize = 8;
pub const TIME_OPTIONS: [u32; 3] = [15, 30, 60];
pub const WORD_OPTIONS: [u32; 4] = [10, 25, 50, 100];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Practice,
    Results,
    History,
}

pub struct App {
    pub screen: Screen,
    pub mode_index: usize,
    pub time_index: usize,
    pub word_index: usize,
    pub punctuation: bool,
    pub numbers: bool,
    pub extended_words: bool,
    pub menu_focus: usize,
    pub results_focus: usize,
    pub session: Option<TypingSession>,
    pub last_result: Option<SessionRecord>,
    pub new_best: bool,
    /// When the results screen was shown, for the reveal animations.
    pub result_shown_at: Option<std::time::Instant>,
    pub history: Vec<SessionRecord>,
    pub status: Option<String>,
    storage: Storage,
    should_quit: bool,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let storage = Storage::new()?;
        let (history, status) = match storage.load_history() {
            Ok(history) => (history, None),
            Err(error) => (
                Vec::new(),
                Some(format!("Could not read local history: {error}")),
            ),
        };
        let settings = storage.load_settings().unwrap_or_default();

        Ok(Self {
            screen: Screen::Menu,
            mode_index: settings.mode_index.min(PracticeMode::ALL.len() - 1),
            time_index: settings.time_index.min(TIME_OPTIONS.len() - 1),
            word_index: settings.word_index.min(WORD_OPTIONS.len() - 1),
            punctuation: settings.punctuation,
            numbers: settings.numbers,
            extended_words: settings.extended_words,
            menu_focus: 0,
            results_focus: 0,
            session: None,
            last_result: None,
            new_best: false,
            result_shown_at: None,
            history,
            status,
            storage,
            should_quit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| ui::draw(frame, self))?;

            if event::poll(Duration::from_millis(33))? {
                match event::read()? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => self.handle_key(key),
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }

            if let Some(session) = self.session.as_mut()
                && self.screen == Screen::Practice
            {
                session.tick(std::time::Instant::now());
                if session.is_finished() {
                    self.complete_session();
                }
            }
        }
        Ok(())
    }

    pub fn selected_mode(&self) -> PracticeMode {
        PracticeMode::ALL[self.mode_index]
    }

    pub fn selected_amount(&self) -> u32 {
        match self.selected_mode() {
            PracticeMode::Time => TIME_OPTIONS[self.time_index],
            PracticeMode::Words => WORD_OPTIONS[self.word_index],
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match self.screen {
            Screen::Menu => self.handle_menu_key(key),
            Screen::Practice => self.handle_practice_key(key),
            Screen::Results => self.handle_results_key(key),
            Screen::History => self.handle_history_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::BackTab => {
                self.menu_focus = self.menu_focus.checked_sub(1).unwrap_or(MENU_ROWS - 1);
            }
            KeyCode::Down | KeyCode::Tab => self.menu_focus = (self.menu_focus + 1) % MENU_ROWS,
            KeyCode::Left => self.change_setting(false),
            KeyCode::Right => self.change_setting(true),
            KeyCode::Enter => match self.menu_focus {
                0..=4 => self.change_setting(true),
                5 => self.start_session(),
                6 => self.screen = Screen::History,
                7 => self.should_quit = true,
                _ => {}
            },
            KeyCode::Esc => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_practice_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.session = None;
            self.screen = Screen::Menu;
            return;
        }
        if key.code == KeyCode::Tab {
            self.restart_session();
            return;
        }

        let Some(session) = self.session.as_mut() else {
            return;
        };

        match key.code {
            KeyCode::Backspace => session.backspace(std::time::Instant::now()),
            KeyCode::Char(value)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                session.type_char(value, std::time::Instant::now());
            }
            _ => {}
        }

        if session.is_finished() {
            self.complete_session();
        }
    }

    fn handle_results_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => self.start_session(),
            KeyCode::Up | KeyCode::BackTab => {
                self.results_focus = self.results_focus.checked_sub(1).unwrap_or(3);
            }
            KeyCode::Down => {
                self.results_focus = (self.results_focus + 1) % 4;
            }
            KeyCode::Enter => match self.results_focus {
                0 => self.start_session(),
                1 => self.screen = Screen::Menu,
                2 => self.screen = Screen::History,
                3 => self.should_quit = true,
                _ => {}
            },
            KeyCode::Esc => self.screen = Screen::Menu,
            _ => {}
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Backspace) {
            self.screen = Screen::Menu;
        }
    }

    fn change_setting(&mut self, forward: bool) {
        match self.menu_focus {
            0 => self.mode_index = cycle(self.mode_index, PracticeMode::ALL.len(), forward),
            1 => match self.selected_mode() {
                PracticeMode::Time => {
                    self.time_index = cycle(self.time_index, TIME_OPTIONS.len(), forward)
                }
                PracticeMode::Words => {
                    self.word_index = cycle(self.word_index, WORD_OPTIONS.len(), forward)
                }
            },
            2 => self.punctuation = !self.punctuation,
            3 => self.numbers = !self.numbers,
            4 => self.extended_words = !self.extended_words,
            _ => {}
        }
        if self.menu_focus < 5 {
            self.save_settings();
        }
    }

    fn save_settings(&mut self) {
        let settings = Settings {
            mode_index: self.mode_index,
            time_index: self.time_index,
            word_index: self.word_index,
            punctuation: self.punctuation,
            numbers: self.numbers,
            extended_words: self.extended_words,
        };
        if let Err(error) = self.storage.save_settings(&settings)
            && self.status.is_none()
        {
            self.status = Some(format!("Could not save settings: {error}"));
        }
    }

    fn start_session(&mut self) {
        let mode = self.selected_mode();
        let amount = self.selected_amount();
        let word_count = match mode {
            PracticeMode::Time => 400,
            PracticeMode::Words => amount as usize,
        };
        let passage = words::generate(
            word_count,
            self.punctuation,
            self.numbers,
            self.extended_words,
        );
        let target = if mode == PracticeMode::Words {
            format!("{passage} ")
        } else {
            passage
        };
        self.session = Some(TypingSession::new(target, mode, amount));
        self.screen = Screen::Practice;
        self.status = None;
    }

    /// Restart with a fresh passage and identical settings, without stopping.
    fn restart_session(&mut self) {
        if self.session.is_some() {
            self.start_session();
        }
    }

    fn complete_session(&mut self) {
        let Some(session) = self.session.take() else {
            return;
        };
        let record = session.record();
        let previous_best = self
            .history
            .iter()
            .filter(|item| item.mode == record.mode && item.amount == record.amount)
            .map(|item| item.wpm)
            .fold(0.0, f64::max);
        self.new_best = record.wpm > previous_best && record.wpm > 0.0;
        self.history.push(record.clone());
        if let Err(error) = self.storage.save_history(&self.history) {
            self.status = Some(format!("Could not save local history: {error}"));
        }
        self.last_result = Some(record);
        self.result_shown_at = Some(std::time::Instant::now());
        self.results_focus = 0;
        self.screen = Screen::Results;
    }
}

fn cycle(current: usize, length: usize, forward: bool) -> usize {
    if forward {
        (current + 1) % length
    } else {
        current.checked_sub(1).unwrap_or(length - 1)
    }
}
