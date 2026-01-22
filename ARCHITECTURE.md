# Architecture

This document describes the architecture of the Fractal Explorer, a Rust application for rendering and interactively exploring the Mandelbrot set.

## Overview

The application supports two modes:
- **CLI mode**: Generates a single fractal image as a PPM file
- **GUI mode**: Interactive exploration with zoom, pan, and real-time parameter adjustment

The codebase follows **hexagonal architecture** (ports & adapters), separating domain logic from external concerns through trait-based abstractions.

## Directory Structure

```
src/
├── bin/
│   └── gui.rs                    # GUI binary entry point
├── controllers/
│   ├── interactive/              # GUI controller with background rendering
│   └── mandelbrot.rs             # CLI controller for batch rendering
├── core/
│   ├── actions/                  # Use cases (fractal generation, pixel mapping)
│   ├── data/                     # Domain types (Complex, PixelBuffer, etc.)
│   ├── fractals/mandelbrot/      # Mandelbrot algorithm and colour maps
│   └── util/                     # Coordinate mapping utilities
├── input/
│   └── gui/                      # GUI event loop and UI state
├── presenters/
│   └── pixels_presenter.rs       # wgpu-based framebuffer renderer
├── storage/
│   └── write_ppm.rs              # PPM file output
├── lib.rs
└── main.rs                       # CLI entry point
```

## Core Domain

### Data Types

| Type | Description |
|------|-------------|
| `Complex` | Complex number with arithmetic operators |
| `Point` | 2D integer pixel coordinates |
| `PixelRect` | Rectangular region in pixel space (inclusive bounds) |
| `ComplexRect` | Rectangular region in the complex plane |
| `Colour` | RGB triplet (u8, u8, u8) |
| `PixelBuffer` | Validated container of RGB bytes |

### Ports (Traits)

**`FractalAlgorithm`** - Computes iteration count for a pixel:
```rust
fn compute(&self, pixel: Point) -> Result<u32, Error>;
```

**`ColourMap<T>`** - Maps algorithm output to colours:
```rust
fn map(&self, value: T) -> Result<Colour, Error>;
```

**`CancelToken`** - Cooperative cancellation:
```rust
fn is_cancelled(&self) -> bool;
```

**`PresenterPort`** - Receives rendered frames:
```rust
fn present(&self, event: RenderEvent);
```

## Rendering Pipeline

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  PixelRect +    │     │  Vec<u32>       │     │  PixelBuffer    │
│  Algorithm      │────▶│  (iterations)   │────▶│  (RGB bytes)    │
│                 │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
   Fractal Gen            Colour Mapping          Presentation
```

1. **Fractal Generation**: Iterate over pixels, compute escape iterations
2. **Colour Mapping**: Transform iteration counts to RGB values
3. **Presentation**: Output to file (CLI) or framebuffer (GUI)

### Parallelization

Three strategies in `core/actions/generate_fractal/`:
- `generate_fractal_parallel_rayon`: Work-stealing via rayon (default)
- `generate_fractal_parallel_arc`: Manual Arc-based thread coordination
- `generate_fractal_serial`: Single-threaded for comparison

Cancellation is checked every 1024 pixels to amortize overhead.

## Controllers

### CLI Controller (`mandelbrot_controller`)

Synchronous single-shot rendering:
1. Configure algorithm parameters
2. Generate fractal data
3. Map to colours
4. Write PPM file

### Interactive Controller

Manages background rendering for responsive GUI:

```
┌─────────────┐    request    ┌─────────────┐    event     ┌─────────────┐
│   GUI       │──────────────▶│   Worker    │─────────────▶│  Presenter  │
│   Thread    │               │   Thread    │              │             │
└─────────────┘               └─────────────┘              └─────────────┘
```

**Key features:**
- **Generation IDs**: Monotonic counter tracks request versions
- **Request coalescing**: New requests replace pending ones (no queue buildup)
- **Stale detection**: Results discarded if generation superseded
- **Silent cancellation**: No error events for user-initiated interrupts

## GUI Architecture

Built on:
- `winit`: Window management and event loop
- `pixels`: wgpu-based framebuffer
- `egui`: Immediate-mode UI overlay

### UI State

Tracks:
- Current view region (ComplexRect)
- Max iterations
- Selected colour map
- Render statistics (duration, generation ID)
- Error messages

### Event Flow

```
User Input → winit Event → UI State Update → Render Request → Worker → Frame
```

## Colour Mapping System

### Available Maps

| Kind | Description |
|------|-------------|
| `FireGradient` | Black → red → yellow → white heat ramp |
| `BlueWhiteGradient` | Dark blue → white gradient |

### Extension

1. Add variant to `MandelbrotColourMapKinds`
2. Implement `MandelbrotColourMap` trait
3. Register in `mandelbrot_colour_map_factory()`

## Error Handling

Errors distinguish between cancellation and true failures:

```rust
enum GenerateFractalError<E> {
    Cancelled,
    Algorithm(E),
}
```

This allows callers to treat cancellation as control flow rather than error.

## File Output

**PPM Format (P6 binary)**:
```
P6
{width} {height}
255
{raw RGB bytes}
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `rayon` | Data parallelism |
| `winit` | Window management (GUI) |
| `pixels` | wgpu framebuffer (GUI) |
| `egui` | UI toolkit (GUI) |

## Testing

87 tests covering:
- Complex number arithmetic
- Rectangle operations
- Pixel-to-complex coordinate mapping
- Fractal algorithm correctness
- Colour mapping
- Cancellation behaviour
- Error handling edge cases

Run with `cargo test`.
