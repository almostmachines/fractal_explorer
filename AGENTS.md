# Repository Guidelines

## Project Description

A Rust-based Mandelbrot fractal renderer with both CLI and interactive GUI capabilities. Features parallel rendering, multiple colour maps, and real-time exploration.

## Project Structure & Module Organization

- `src/core/`: domain logic (fractal algorithms, data types, actions, utils). Keep this layer free of UI/IO.
- `src/controllers/`: orchestration for CLI/interactive flows plus ports (interfaces) for presenters.
- `src/presenters/`: output adapters (e.g., `presenters/file/ppm.rs` for PPM files, `presenters/pixels/` for GUI pixels).
- `src/input/gui/`: GUI app and command wiring, compiled only with the `gui` feature.
- Entry points:
  - `src/main.rs`: CLI “test” generator that writes `output/mandelbrot.ppm`.
  - `src/bin/gui.rs`: GUI binary (feature-gated).
- Repo utilities:
  - `plans/`: design notes/working plans.
  - `.beads/`: Beads issue tracking metadata (treat as source-controlled project state).

## Build, Test, and Development Commands

```bash
cargo build                 # compile default (non-GUI) target
cargo run                   # generate output/mandelbrot.ppm
cargo test                  # run unit tests
cargo run --bin gui --features gui   # run GUI app

cargo fmt                   # format (rustfmt)
cargo clippy --all-targets --all-features -- -D warnings  # lint
```

## Coding Style & Naming Conventions

- Rust edition: 2024 (use idiomatic std error types; prefer `Result<T, E>` with explicit error enums).
- Formatting: `cargo fmt` (4-space indent; rustfmt defaults).
- Naming: modules/functions `snake_case`, types/traits `PascalCase`, constants `SCREAMING_SNAKE_CASE`.
- Keep GUI-only code behind `cfg(feature = "gui")` and avoid leaking GUI types into `src/core/`.

## Testing Guidelines

- Use Rust’s built-in test harness (`#[cfg(test)] mod tests { ... }`) colocated with the code under test.
- Prefer fast, deterministic unit tests for `src/core/` invariants (e.g., `PixelRect`, mapping, algorithms).
- If a change affects GUI state logic, add/extend tests in `src/input/gui/app/`.

## Commit & Pull Request Guidelines

- Commit messages follow a short, imperative style (examples in history: “Refactor …”, “Add …”, “Update docs”).
- Keep commits focused; avoid mixing formatting-only changes with behavior changes.
- PRs should include: what/why, how to run (commands), and artifacts when relevant:
  - CLI changes: attach or describe expected `output/mandelbrot.ppm`.
  - GUI changes: screenshots or a short screen recording.
- Beads: prefer `bd create/list/show/update/sync`; don’t manually edit `.beads/issues.jsonl`.


## GUI Run Permissions
- When running the GUI app (for example `cargo run --features gui --bin gui`), **always request escalated permissions**.
  - Reason: the sandbox seccomp profile blocks Wayland socket connections which triggers `WaylandError(Connection(NoCompositor))` even when Wayland is available.
  - Alternatively: run the session in `danger-full-access` mode so GUI commands can execute without needing per-command escalation.

## GUI Visual Testing
- visually test the GUI by taking screenshots with `grim` (Wayland).

```bash
# Build and run GUI in background
cargo run --bin gui --features gui &
GUI_PID=$!

# Wait for window to render
sleep 2

# Capture screenshot
grim /tmp/fractal_gui_test.png

# Kill the GUI
kill $GUI_PID

# Read the screenshot at /tmp/fractal_gui_test.png
```

## Task Tracking and Management

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

### Quick Reference

```bash
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Full issue details with dependencies
bd create --title="..." --type=task|feature|bug --priority=2
bd update <id> --status=in_progress
bd close <id> --reason="Completed"
bd close <id1> <id2>  # Close multiple issues at once
bd sync               # Commit and push changes
```

### Workflow Pattern

**Important**. If you're working with beads tasks, you must follow this work flow.

1. **Start**: Run `bd ready` to find actionable work.
2. **Claim**: Use `bd update <id> --status=in_progress`.
3. **Work**: Implement the issue and test.
4. **Discovery**: If you discover new work, create a new bead with discovered-from:<parent-id>.
5. **Complete**: Mark the issue as closed with `bd close <id>`. Commit and push your work.
6. **Sync**: Always run `bd sync` after marking an issue as complete, even if you didn't commit any work.

### Landing the Plane (Session Completion)

When ending a work session, complete all steps below.

1. **File issues for remaining work** - Create issues for anything that needs follow-up.
2. **Run quality gates** (if code changed) - Tests, linters, builds.
3. **Update issue status** - Close finished work, update in-progress items.
4. **Push to remote:**
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Verify** - All changes committed AND pushed.
6. **Hand off** - Provide context for next session.

### Best Practices

- Check `bd ready` at session start to find available work.
- Update status as you work (in_progress → closed).
- Create new issues with `bd create` when you discover tasks.
- Use descriptive titles and set appropriate priority/type.
- Always `bd sync` before ending session.

## Generating tree diagrams

You can use the command `tree --gitignore`.
