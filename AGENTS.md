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

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
