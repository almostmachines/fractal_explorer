use crate::{core::data::pixel_buffer::PixelBuffer, input::gui::app::frame_overlay::FrameOverlay};

const GLYPH_WIDTH_CELLS: u32 = 5;
const GLYPH_HEIGHT_CELLS: u32 = 7;
const GLYPH_GAP_CELLS: u32 = 1;
const BACKPLATE_PAD_X_CELLS: u32 = 2;
const BACKPLATE_PAD_Y_CELLS: u32 = 2;
const TARGET_BACKPLATE_WIDTH_NUMERATOR: u32 = 2;
const TARGET_BACKPLATE_WIDTH_DENOMINATOR: u32 = 3;
const MAX_TEXT_HEIGHT_NUMERATOR: u32 = 1;
const MAX_TEXT_HEIGHT_DENOMINATOR: u32 = 3;
const HELP_TEXT_SCALE_NUMERATOR: u32 = 1;
const HELP_TEXT_SCALE_DENOMINATOR: u32 = 3;
const HELP_TEXT_TOP_GAP_CELLS: u32 = 14;
const HELP_TEXT_LINE_GAP_CELLS: u32 = 6;

type Glyph = [u8; GLYPH_HEIGHT_CELLS as usize];

const GLYPH_SPACE: Glyph = [0b00000; GLYPH_HEIGHT_CELLS as usize];
const GLYPH_A: Glyph = [
    0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
];
const GLYPH_C: Glyph = [
    0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
];
const GLYPH_D: Glyph = [
    0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
];
const GLYPH_E: Glyph = [
    0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
];
const GLYPH_F: Glyph = [
    0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
];
const GLYPH_G: Glyph = [
    0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111,
];
const GLYPH_H: Glyph = [
    0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
];
const GLYPH_I: Glyph = [
    0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
];
const GLYPH_L: Glyph = [
    0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
];
const GLYPH_N: Glyph = [
    0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
];
const GLYPH_O: Glyph = [
    0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
];
const GLYPH_P: Glyph = [
    0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
];
const GLYPH_R: Glyph = [
    0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
];
const GLYPH_S: Glyph = [
    0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
];
const GLYPH_T: Glyph = [
    0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
];
const GLYPH_U: Glyph = [
    0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
];
const GLYPH_W: Glyph = [
    0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
];
const GLYPH_SLASH: Glyph = [
    0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
];
const GLYPH_COLON: Glyph = [
    0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
];

const PAUSED_GLYPHS: [Glyph; 6] = [
    GLYPH_P, GLYPH_A, GLYPH_U, GLYPH_S, GLYPH_E, GLYPH_D,
];

const HELP_TEXT_LINES: [&str; 3] = [
    "WASD: Up/Left/Down/Right",
    "Up/down arrow: Accelerate/Deccelerate",
    "P: Pause/Unpause",
];

const PALETTE: [[u8; PixelBuffer::BYTES_PER_PIXEL]; 5] = [
    [88, 6, 0, PixelBuffer::ALPHA_OPAQUE],
    [168, 30, 0, PixelBuffer::ALPHA_OPAQUE],
    [230, 88, 8, PixelBuffer::ALPHA_OPAQUE],
    [255, 166, 48, PixelBuffer::ALPHA_OPAQUE],
    [255, 232, 180, PixelBuffer::ALPHA_OPAQUE],
];

const BACKPLATE_COLOUR: [u8; PixelBuffer::BYTES_PER_PIXEL] = [6, 4, 10, 176];
const HELP_TEXT_COLOUR: [u8; PixelBuffer::BYTES_PER_PIXEL] = PALETTE[3];

const WORD_LEN: u32 = PAUSED_GLYPHS.len() as u32;
const WORD_COLS: u32 =
    WORD_LEN * GLYPH_WIDTH_CELLS + (WORD_LEN.saturating_sub(1)) * GLYPH_GAP_CELLS;
const WORD_ROWS: u32 = GLYPH_HEIGHT_CELLS;
const HELP_LINE_COUNT: usize = HELP_TEXT_LINES.len();

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Rect {
    fn right(&self) -> u32 {
        self.x + self.width
    }

    fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OverlayLayout {
    cell_size: u32,
    help_cell_size: u32,
    word_bounds: Rect,
    help_line_bounds: [Rect; HELP_LINE_COUNT],
    backplate_bounds: Rect,
}

pub fn draw_frame_overlay(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    overlay: &FrameOverlay,
) {
    if !overlay.paused {
        return;
    }

    let Some(layout) = compute_layout(frame_width, frame_height) else {
        return;
    };

    debug_assert_eq!(
        frame.len(),
        (frame_width as usize)
            .saturating_mul(frame_height as usize)
            .saturating_mul(PixelBuffer::BYTES_PER_PIXEL)
    );

    fill_rect(
        frame,
        frame_width,
        frame_height,
        layout.backplate_bounds,
        BACKPLATE_COLOUR,
    );

    let mut glyph_left = layout.word_bounds.x;

    for (glyph_index, glyph) in PAUSED_GLYPHS.iter().enumerate() {
        draw_word_glyph(
            frame,
            frame_width,
            frame_height,
            layout.cell_size,
            glyph_left,
            layout.word_bounds.y,
            glyph,
            glyph_index,
        );

        glyph_left += GLYPH_WIDTH_CELLS * layout.cell_size;
        if glyph_index + 1 < PAUSED_GLYPHS.len() {
            glyph_left += GLYPH_GAP_CELLS * layout.cell_size;
        }
    }

    for (line_index, text) in HELP_TEXT_LINES.iter().enumerate() {
        draw_help_text_line(
            frame,
            frame_width,
            frame_height,
            layout.help_cell_size,
            layout.help_line_bounds[line_index],
            text,
        );
    }
}

fn compute_layout(frame_width: u32, frame_height: u32) -> Option<OverlayLayout> {
    if frame_width == 0 || frame_height == 0 {
        return None;
    }

    let target_backplate_width = frame_width
        .saturating_mul(TARGET_BACKPLATE_WIDTH_NUMERATOR)
        / TARGET_BACKPLATE_WIDTH_DENOMINATOR;
    let target_backplate_height = frame_height
        .saturating_mul(MAX_TEXT_HEIGHT_NUMERATOR)
        / MAX_TEXT_HEIGHT_DENOMINATOR;
    let mut best_within_target = None;
    let mut best_that_fits = None;

    for cell_size in 1..=frame_width.min(frame_height) {
        let layout = layout_for_cell_size(frame_width, frame_height, cell_size);
        if layout.backplate_bounds.width > frame_width
            || layout.backplate_bounds.height > frame_height
        {
            break;
        }

        best_that_fits = Some(layout);
        if layout.backplate_bounds.width <= target_backplate_width
            && layout.backplate_bounds.height <= target_backplate_height
        {
            best_within_target = Some(layout);
        }
    }

    best_within_target.or(best_that_fits)
}

fn layout_for_cell_size(frame_width: u32, frame_height: u32, cell_size: u32) -> OverlayLayout {
    let help_cell_size = help_cell_size(cell_size);
    let word_width = WORD_COLS * cell_size;
    let word_height = WORD_ROWS * cell_size;
    let help_line_widths = HELP_TEXT_LINES.map(|text| text_width_pixels(text, help_cell_size));
    let help_width = help_line_widths.into_iter().max().unwrap_or(0);
    let help_line_height = GLYPH_HEIGHT_CELLS * help_cell_size;
    let help_line_gap = HELP_TEXT_LINE_GAP_CELLS * help_cell_size;
    let help_top_gap = HELP_TEXT_TOP_GAP_CELLS * help_cell_size;
    let help_height = (HELP_LINE_COUNT as u32).saturating_mul(help_line_height)
        + (HELP_LINE_COUNT as u32)
            .saturating_sub(1)
            .saturating_mul(help_line_gap);
    let content_width = word_width.max(help_width);
    let content_height = word_height + help_top_gap + help_height;
    let backplate_width = content_width + (BACKPLATE_PAD_X_CELLS * 2 * cell_size);
    let backplate_height = content_height + (BACKPLATE_PAD_Y_CELLS * 2 * cell_size);
    let backplate_bounds = Rect {
        x: frame_width.saturating_sub(backplate_width) / 2,
        y: frame_height.saturating_sub(backplate_height) / 2,
        width: backplate_width,
        height: backplate_height,
    };
    let content_left = backplate_bounds.x + (BACKPLATE_PAD_X_CELLS * cell_size);
    let word_bounds = Rect {
        x: content_left + (content_width - word_width) / 2,
        y: backplate_bounds.y + (BACKPLATE_PAD_Y_CELLS * cell_size),
        width: word_width,
        height: word_height,
    };
    let mut help_line_bounds = [Rect::default(); HELP_LINE_COUNT];
    let mut line_top = word_bounds.bottom() + help_top_gap;

    for (index, width) in help_line_widths.into_iter().enumerate() {
        help_line_bounds[index] = Rect {
            x: content_left + (content_width - width) / 2,
            y: line_top,
            width,
            height: help_line_height,
        };

        line_top += help_line_height;
        if index + 1 < HELP_LINE_COUNT {
            line_top += help_line_gap;
        }
    }

    OverlayLayout {
        cell_size,
        help_cell_size,
        word_bounds,
        help_line_bounds,
        backplate_bounds,
    }
}

fn help_cell_size(cell_size: u32) -> u32 {
    let scaled =
        cell_size.saturating_mul(HELP_TEXT_SCALE_NUMERATOR) / HELP_TEXT_SCALE_DENOMINATOR;
    scaled.max(1)
}

fn text_width_pixels(text: &str, cell_size: u32) -> u32 {
    text_cols(text) * cell_size
}

fn text_cols(text: &str) -> u32 {
    let char_count = text.chars().count() as u32;
    if char_count == 0 {
        return 0;
    }

    char_count * GLYPH_WIDTH_CELLS + (char_count - 1) * GLYPH_GAP_CELLS
}

fn draw_word_glyph(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_size: u32,
    left: u32,
    top: u32,
    glyph: &Glyph,
    glyph_index: usize,
) {
    for (row, row_bits) in glyph.iter().enumerate() {
        for col in 0..GLYPH_WIDTH_CELLS as usize {
            let bit = 1 << ((GLYPH_WIDTH_CELLS as usize - 1) - col);
            if row_bits & bit == 0 {
                continue;
            }

            let colour = glyph_cell_colour(glyph_index, row, col);
            fill_rect(
                frame,
                frame_width,
                frame_height,
                Rect {
                    x: left + (col as u32 * cell_size),
                    y: top + (row as u32 * cell_size),
                    width: cell_size,
                    height: cell_size,
                },
                colour,
            );
        }
    }
}

fn draw_help_text_line(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_size: u32,
    bounds: Rect,
    text: &str,
) {
    let mut glyph_left = bounds.x;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(glyph) = help_text_glyph(ch) {
            draw_monochrome_glyph(
                frame,
                frame_width,
                frame_height,
                cell_size,
                glyph_left,
                bounds.y,
                &glyph,
                HELP_TEXT_COLOUR,
            );
        }

        glyph_left += GLYPH_WIDTH_CELLS * cell_size;
        if chars.peek().is_some() {
            glyph_left += GLYPH_GAP_CELLS * cell_size;
        }
    }
}

fn help_text_glyph(ch: char) -> Option<Glyph> {
    match ch.to_ascii_uppercase() {
        ' ' => Some(GLYPH_SPACE),
        ':' => Some(GLYPH_COLON),
        '/' => Some(GLYPH_SLASH),
        'A' => Some(GLYPH_A),
        'C' => Some(GLYPH_C),
        'D' => Some(GLYPH_D),
        'E' => Some(GLYPH_E),
        'F' => Some(GLYPH_F),
        'G' => Some(GLYPH_G),
        'H' => Some(GLYPH_H),
        'I' => Some(GLYPH_I),
        'L' => Some(GLYPH_L),
        'N' => Some(GLYPH_N),
        'O' => Some(GLYPH_O),
        'P' => Some(GLYPH_P),
        'R' => Some(GLYPH_R),
        'S' => Some(GLYPH_S),
        'T' => Some(GLYPH_T),
        'U' => Some(GLYPH_U),
        'W' => Some(GLYPH_W),
        _ => None,
    }
}

fn draw_monochrome_glyph(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    cell_size: u32,
    left: u32,
    top: u32,
    glyph: &Glyph,
    colour: [u8; PixelBuffer::BYTES_PER_PIXEL],
) {
    for (row, row_bits) in glyph.iter().enumerate() {
        for col in 0..GLYPH_WIDTH_CELLS as usize {
            let bit = 1 << ((GLYPH_WIDTH_CELLS as usize - 1) - col);
            if row_bits & bit == 0 {
                continue;
            }

            fill_rect(
                frame,
                frame_width,
                frame_height,
                Rect {
                    x: left + (col as u32 * cell_size),
                    y: top + (row as u32 * cell_size),
                    width: cell_size,
                    height: cell_size,
                },
                colour,
            );
        }
    }
}

fn glyph_cell_colour(glyph_index: usize, row: usize, col: usize) -> [u8; PixelBuffer::BYTES_PER_PIXEL] {
    PALETTE[(glyph_index + col + (row * 2)) % PALETTE.len()]
}

fn fill_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    rect: Rect,
    colour: [u8; PixelBuffer::BYTES_PER_PIXEL],
) {
    let x_end = rect.right().min(frame_width);
    let y_end = rect.bottom().min(frame_height);
    let stride = frame_width as usize * PixelBuffer::BYTES_PER_PIXEL;

    for y in rect.y..y_end {
        let row_start = y as usize * stride;
        for x in rect.x..x_end {
            let index = row_start + x as usize * PixelBuffer::BYTES_PER_PIXEL;
            blend_pixel(&mut frame[index..index + PixelBuffer::BYTES_PER_PIXEL], colour);
        }
    }
}

fn blend_pixel(pixel: &mut [u8], colour: [u8; PixelBuffer::BYTES_PER_PIXEL]) {
    let alpha = u16::from(colour[3]);

    if alpha == u16::from(PixelBuffer::ALPHA_OPAQUE) {
        pixel.copy_from_slice(&colour);
        return;
    }

    let inverse_alpha = 255_u16.saturating_sub(alpha);
    for channel in 0..3 {
        let dst = u16::from(pixel[channel]);
        let src = u16::from(colour[channel]);
        pixel[channel] = ((dst * inverse_alpha + src * alpha + 127) / 255) as u8;
    }

    pixel[3] = PixelBuffer::ALPHA_OPAQUE;
}

#[cfg(test)]
mod tests {
    use super::{
        BACKPLATE_PAD_X_CELLS, BACKPLATE_PAD_Y_CELLS, MAX_TEXT_HEIGHT_DENOMINATOR,
        MAX_TEXT_HEIGHT_NUMERATOR, TARGET_BACKPLATE_WIDTH_DENOMINATOR,
        TARGET_BACKPLATE_WIDTH_NUMERATOR, WORD_COLS, WORD_ROWS, compute_layout,
        draw_frame_overlay,
    };
    use crate::{core::data::pixel_buffer::PixelBuffer, input::gui::app::frame_overlay::FrameOverlay};

    fn solid_frame(width: u32, height: u32, rgba: [u8; PixelBuffer::BYTES_PER_PIXEL]) -> Vec<u8> {
        let mut frame =
            vec![0; width as usize * height as usize * PixelBuffer::BYTES_PER_PIXEL];
        for pixel in frame.chunks_exact_mut(PixelBuffer::BYTES_PER_PIXEL) {
            pixel.copy_from_slice(&rgba);
        }
        frame
    }

    #[test]
    fn layout_dimensions_match_overlay_content() {
        let layout = compute_layout(800, 600).expect("layout should fit");

        assert_eq!(layout.word_bounds.width, WORD_COLS * layout.cell_size);
        assert_eq!(layout.word_bounds.height, WORD_ROWS * layout.cell_size);
        assert!(layout.help_cell_size < layout.cell_size);
        assert_eq!(
            layout.word_bounds.x,
            layout.backplate_bounds.x
                + (layout.backplate_bounds.width - layout.word_bounds.width) / 2
        );
        assert_eq!(
            layout.backplate_bounds.width,
            layout.word_bounds.width.max(
                layout
                    .help_line_bounds
                    .iter()
                    .map(|bounds| bounds.width)
                    .max()
                    .unwrap_or(0)
            ) + (BACKPLATE_PAD_X_CELLS * 2 * layout.cell_size)
        );
        assert!(
            layout.help_line_bounds[0].y >= layout.word_bounds.bottom() + layout.help_cell_size
        );
        assert_eq!(
            layout.backplate_bounds.height,
            layout.help_line_bounds[2].bottom() - layout.backplate_bounds.y
                + (BACKPLATE_PAD_Y_CELLS * layout.cell_size)
        );
    }

    #[test]
    fn layout_backplate_is_centered_in_frame() {
        let frame_width = 801;
        let frame_height = 601;
        let layout = compute_layout(frame_width, frame_height).expect("layout should fit");

        let right_margin = frame_width - layout.backplate_bounds.right();
        let bottom_margin = frame_height - layout.backplate_bounds.bottom();

        assert!(layout.backplate_bounds.x.abs_diff(right_margin) <= 1);
        assert!(layout.backplate_bounds.y.abs_diff(bottom_margin) <= 1);
    }

    #[test]
    fn scale_clamps_down_on_tiny_windows() {
        assert_eq!(compute_layout(60, 40), None);
    }

    #[test]
    fn scale_is_limited_by_frame_height_on_large_shallow_windows() {
        let layout = compute_layout(1_600, 200).expect("layout should fit");

        assert_eq!(layout.cell_size, 2);
    }

    #[test]
    fn backplate_stays_within_target_bounds_on_wide_viewports() {
        let frame_width = 1_114;
        let frame_height = 768;
        let layout = compute_layout(frame_width, frame_height).expect("layout should fit");

        assert!(
            layout.backplate_bounds.width
                <= frame_width * TARGET_BACKPLATE_WIDTH_NUMERATOR
                    / TARGET_BACKPLATE_WIDTH_DENOMINATOR
        );
        assert!(
            layout.backplate_bounds.height
                <= frame_height * MAX_TEXT_HEIGHT_NUMERATOR / MAX_TEXT_HEIGHT_DENOMINATOR
        );
    }

    #[test]
    fn rasterization_only_changes_pixels_inside_backplate_bounds() {
        let frame_width = 320;
        let frame_height = 240;
        let layout = compute_layout(frame_width, frame_height).expect("layout should fit");
        let mut frame = solid_frame(frame_width, frame_height, [25, 40, 60, 255]);
        let before = frame.clone();

        draw_frame_overlay(&mut frame, frame_width, frame_height, &FrameOverlay { paused: true });

        let mut changed_pixels = 0_usize;
        for y in 0..frame_height {
            for x in 0..frame_width {
                let index = ((y * frame_width + x) as usize) * PixelBuffer::BYTES_PER_PIXEL;
                if frame[index..index + PixelBuffer::BYTES_PER_PIXEL]
                    != before[index..index + PixelBuffer::BYTES_PER_PIXEL]
                {
                    changed_pixels += 1;
                    assert!(
                        x >= layout.backplate_bounds.x
                            && x < layout.backplate_bounds.right()
                            && y >= layout.backplate_bounds.y
                            && y < layout.backplate_bounds.bottom(),
                        "pixel changed outside backplate at ({x}, {y})"
                    );
                }
            }
        }

        assert!(changed_pixels > 0);
    }

    #[test]
    fn unpaused_overlay_is_a_no_op() {
        let mut frame = solid_frame(240, 160, [18, 24, 32, 255]);
        let before = frame.clone();

        draw_frame_overlay(&mut frame, 240, 160, &FrameOverlay::default());

        assert_eq!(frame, before);
    }
}
