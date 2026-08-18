# Contributing to Terminal Typer

Thanks for taking the time to contribute.

## Before you start

Please search existing issues and pull requests before opening a new one. For
larger changes, open an issue first so the intended behavior can be discussed
before implementation.

## Development setup

Terminal Typer is a Rust terminal application. Install Rust 1.85 or newer, then
run:

~~~bash
cargo run --release
~~~

## Checks before opening a pull request

Run the full local verification set:

~~~bash
cargo fmt -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
~~~

Pull requests should:

- Explain the user-facing or maintenance change.
- Include focused tests when behavior changes.
- Keep generated output, local configuration, and user data out of commits.
- Update documentation when setup or behavior changes.
- Avoid unrelated formatting or dependency churn.

## Commit and review expectations

Keep commits small enough to review and describe the behavior they change.
Maintainers may ask for revisions to clarify scope, tests, or documentation.
By submitting a contribution, you agree that it may be distributed under the
project's MIT license, subject to the third-party notices in this repository.
