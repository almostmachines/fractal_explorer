# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Rust crate sources.
  - `src/main.rs` is the CLI entry point; it currently renders a Mandelbrot image.
  - `src/lib.rs` exposes the public library API.
  - `src/controllers/` wires inputs, rendering, and output.
  - `src/core/` contains fractal algorithms, data types, and rendering actions.
  - `src/storage/` writes output formats (currently PPM).
- `output/` contains generated images (for example `output/mandelbrot.ppm`).
- `target/` is Cargo build output; do not edit or commit it.

## Build, Test, and Development Commands
- `cargo build` — compile the project.
- `cargo run` — run the default Mandelbrot render (writes to `output/`).
- `cargo test` — execute unit tests embedded in modules.
- `cargo fmt` — format code with rustfmt.
- `cargo clippy` — run lint checks.
- `cargo llvm-cov` — optional coverage report (requires the tool to be installed).

## Coding Style & Naming Conventions
- Follow standard Rust style (rustfmt defaults, 4-space indentation).
- Use `snake_case` for functions/modules, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused: algorithms in `core/fractals`, shared data in `core/data`, and utilities in `core/util`.

## Testing Guidelines
- Tests live next to the code under `#[cfg(test)]` blocks.
- Prefer small, deterministic unit tests that exercise core math and pixel operations.
- Name tests descriptively (for example `test_generate_fractal_returns_ok`).
- Run all tests with `cargo test` before opening a PR.

## Commit & Pull Request Guidelines
- Commit subjects are short, sentence-case, imperative (examples from history: `Rename fractal generation actions`).
- Keep commits scoped; avoid mixing refactors with feature changes.
- PRs should include: a concise summary, tests run, and sample output images or notes when rendering changes (`output/*.ppm`).

## Configuration & Output Notes
- Rendering defaults (size, iterations, output path) are set in `src/controllers/mandelbrot.rs`.
- Generated files should be kept in `output/` and should not be used as inputs.

## Tips
- To generate tree diagrams you can use the command `tree --gitignore`.

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

1. **Start**: Run `bd ready` to find actionable work.
2. **Claim**: Use `bd update <id> --status=in_progress`.
3. **Work**: Implement the issue and test.
4. **Discovery**: If you discover new work, create a new bead with discovered-from:<parent-id>.
5. **Complete**: Mark the issue as closed with `bd close <id>`. Commit and push your work. Run `bd sync`.
6. **Sync**: Always run `bd sync` at session end.

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
