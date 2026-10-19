# Terminal Typer

> A focused, local-first typing practice instrument for the terminal.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![TUI](https://img.shields.io/badge/TUI-Ratatui-green.svg)](https://ratatui.rs)

Terminal Typer is a keyboard-driven typing practice tool designed directly for the terminal. Built in Rust with [Ratatui](https://github.com/ratatui/ratatui) and [Crossterm](https://github.com/crossterm-rs/crossterm), it provides instantaneous feedback, detailed post-session analytics, and local persistence without requiring accounts, telemetry, or network connectivity.

---

## Features

- **Practice Modes:**
  - **Timed Practice:** 15, 30, or 60 seconds
  - **Word Practice:** 10, 25, 50, or 100 words
- **Corpus Selection:**
  - Standard Monkeytype English corpus (~200 most frequent words)
  - Extended English 1k corpus (~1,000 most frequent words)
  - Optional punctuation (commas, periods, question marks, quotes, hyphens)
  - Optional number insertions (digits, years, percentages, currencies)
- **Live HUD & Metrics:**
  - Real-time Net Words Per Minute (WPM)
  - Real-time Raw WPM (including error keystrokes)
  - Character accuracy percentage
  - Real-time error counter & countdown / word progress indicator
- **Results & Analytics:**
  - Per-second WPM progression sparkline / bar graph
  - Consistency rating (derived from speed variation across the session)
  - Automated Personal Best (PB) detection per mode and length
  - Character breakdowns (correct, errors, total keystrokes, elapsed milliseconds)
- **Fluid Typing Ergonomics:**
  - Three-line scrolling window with the active line centered (Monkeytype-style)
  - Non-blocking space-commit word mechanics (space advances past typos without stalling)
  - Single-keystroke rewind into committed words via backspace
  - Instant session restart (`Tab`) mid-test or on results screen
- **Local Persistence:**
  - Zero cloud dependencies. Stores the 200 most recent sessions in local JSON
  - Remembers user preferences between application launches
  - Atomic file writes to prevent data corruption during sudden termination
- **Terminal Compatibility:**
  - Designed for standard `80x24` terminal dimensions with graceful compact fallback views
  - Honors native terminal palette, transparency, and typography

---

## Technical Architecture

```
                                +-------------------+
                                |    Terminal UI    |
                                |     (Ratatui)     |
                                +---------^---------+
                                          | Frame Draw
                                          |
+-------------------+             +-------+---------+
|     Raw Input     |             |                 |
|    (Crossterm)    +------------>+   App State /   |
|   30 FPS Poll     |  KeyEvents  | Event Dispatch  |
+-------------------+             |                 |
                                  +---+----+----+---+
                                      |    |    |
           +--------------------------+    |    +--------------------------+
           v                               v                               v
+--------------------+           +--------------------+          +--------------------+
|   Typing Engine    |           |   Corpus Builder   |          |   Local Storage    |
|   (typing.rs)      |           |    (words.rs)      |          |   (storage.rs)     |
|                    |           |                    |          |                    |
| - Caret tracking   |           | - Monkeytype 200   |          | - Atomic save/load |
| - Non-block commit |           | - English 1k list  |          | - history.json     |
| - Timeline samples |           | - Punctuation/Nums |          | - settings.json    |
+--------------------+           +--------------------+          +--------------------+
```

### Core Engine Details

1. **Typing Pipeline (`src/typing.rs`):**
   - **Caret & Input Buffer:** Keystrokes are buffered into a character vector. A dedicated `timeline` vector captures `(elapsed_ms, keystrokes, correct_keystrokes)` snapshots per keypress for micro-second precision analytics.
   - **Space-Commit Semantics:** When pressing `Space` before a word is fully typed, untyped characters are filled with a `SKIPPED` marker (`\u{0}`). This prevents typists from getting stuck on individual typos while ensuring uncorrected characters are penalized in accuracy and scored WPM.
   - **Unwinding Backspace:** When you push `Backspace` on a committed word, TerminalTyper removes all of its `SKIPPED` characters. The caret goes back to the position of the last physical keystroke.

2. **Metrics & Statistical Calculations:**
   - **Net WPM:**
     $$\text{WPM} = \frac{\text{Scored Correct Characters} / 5}{\text{Elapsed Minutes}}$$
     *(In timed mode, correct partial words get credit. In word mode, only complete words without errors get a score.)*
   - **Raw WPM:**
     $$\text{Raw WPM} = \frac{\text{Total Keystrokes} / 5}{\text{Elapsed Minutes}}$$
   - **Accuracy:**
     $$\text{Accuracy (\%)} = \frac{\text{Correct Keystrokes}}{\text{Total Keystrokes}} \times 100$$
   - **Consistency:**
     $$\text{Consistency (\%)} = \max\left(0, 100 \times \left(1 - \frac{\sigma}{\mu}\right)\right)$$
     *where $\mu$ is the mean of the per-second WPM samples and $\sigma$ is the standard deviation.*

3. **Atomic File Persistence (`src/storage.rs`):**
   - TerminalTyper writes user data to a temporary sibling file (`*.tmp`). Then an atomic rename replaces the target file (`history.json` or `settings.json`). If the process stops, the file is not truncated or corrupted.

---

## Controls & Keybindings

| Key | Context | Action |
| --- | --- | --- |
| `Up` / `Down` / `Tab` | Menu | Navigate menu options |
| `Left` / `Right` / `Enter` | Menu | Cycle settings (Mode, Duration, Words, Punctuation, Numbers) |
| `Enter` | Menu (on Start) | Begin practice session |
| `Type characters` | Practice | Type the active passage |
| `Space` | Practice | Commit current word and advance to next |
| `Backspace` | Practice | Erase previous character or unwind committed word |
| `Tab` | Practice / Results | Instantly restart session with fresh passage |
| `Esc` | Practice | Cancel current session and return to menu |
| `Up` / `Down` / `Enter` | Results | Navigate actions (Retry, Menu, History, Quit) |
| `Esc` / `Enter` | History | Return to main menu |
| `Ctrl + C` | Anywhere | Force exit application |

---

## Installation & Requirements

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (version 1.85 or newer)
- A terminal with ANSI color and raw mode support (Windows Terminal, Alacritty, iTerm2, Kitty, WezTerm, etc.)

### Build from Source

```bash
# Clone repository
git clone https://github.com/Hi9841/terminal-typer.git
cd terminal-typer

# Run with release optimizations
cargo run --release
```

### Install Globally via Cargo

```bash
cargo install --path .
```

---

## Storage Locations

Terminal Typer stores configuration and history files in your system's standard application data directory:

- **Windows:** `%LOCALAPPDATA%\TerminalTyper\TerminalTyper\`
- **macOS:** `~/Library/Application Support/dev.TerminalTyper.TerminalTyper/`
- **Linux:** `~/.local/share/TerminalTyper/`

Files created:
- `settings.json` - Stored mode preferences, duration, wordlist, and modifiers.
- `history.json` - Rolling log of the last 200 completed sessions with per-second timeline data.

---

## Development & Verification

Run the test suite, formatter, and linter:

```bash
# Run unit and integration tests
cargo test

# Check code formatting
cargo fmt -- --check

# Run Clippy linter with strict warnings
cargo clippy --all-targets --all-features -- -D warnings
```

---

## License

This project is licensed under the [MIT License](LICENSE).

The built-in word corpus is adapted from Monkeytype and has separate
third-party licensing requirements. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Contributing

Bug reports, documentation improvements, and pull requests are welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the development workflow and checks.
