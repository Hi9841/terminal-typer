# Product

<!-- impeccable:product-schema 1 -->

## Platform

terminal

## Users

People who work in a terminal and want a fast, focused typing practice session without opening a browser.

## Product Purpose

Terminal Typer starts a local typing test, gives immediate feedback while typing, and saves completed-session statistics locally. Success means a user can launch, type, understand the result, and retry without leaving the keyboard.

## Positioning

Typing practice that lives inside the terminal: offline-first, keyboard-only, and intentionally lightweight.

## Operating Context

The app is used in a terminal window, commonly alongside other developer tools. It must remain usable at a compact 80x24 terminal size and must not depend on a browser, network, account, or server for practice.

## Capabilities and Constraints

- Local time and word practice modes.
- Live WPM, accuracy, errors, and progress feedback.
- Backspace correction and Escape to leave an active test.
- Completed sessions are persisted as local JSON history.
- The current practice version has no networking, accounts, matchmaking, or remote database.
- Future hosted multiplayer work is deliberately undecided here.

## Brand Commitments

The product name is Terminal Typer. The user referenced Monkeytype as an
interaction and quality reference. The adapted word corpus is documented in
THIRD_PARTY_NOTICES.md.

## Evidence on Hand

The current repository contains a working Rust practice engine, terminal renderer, local JSON storage, and built-in word corpus. No external brand assets or user research were supplied.

## Product Principles

- Start quickly.
- Keep attention on the text being typed.
- Make performance legible without interrupting flow.
- Keep practice local, predictable, and recoverable.

## Accessibility & Inclusion

The app is keyboard-only by default and should retain clear focus states, readable contrast, and a useful compact-terminal fallback.
