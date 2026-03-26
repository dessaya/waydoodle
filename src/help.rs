use std::io::BufReader;
use std::sync::LazyLock;

use bdf_reader::Font;

use crate::canvas::{Canvas, GLYPH_H, GLYPH_W, Rect};
use crate::waydoodle::ALL_KEYS;

const FONT_REGULAR_BDF: &[u8] = include_bytes!("../assets/Tamzen8x16r.bdf");
const FONT_BOLD_BDF: &[u8] = include_bytes!("../assets/Tamzen8x16b.bdf");

static FONT_REGULAR: LazyLock<Font> = LazyLock::new(|| {
    Font::read(BufReader::new(FONT_REGULAR_BDF)).expect("Failed to parse regular BDF font")
});
static FONT_BOLD: LazyLock<Font> = LazyLock::new(|| {
    Font::read(BufReader::new(FONT_BOLD_BDF)).expect("Failed to parse bold BDF font")
});

const LINE_HEIGHT: u32 = GLYPH_H + 4;
const PADDING: u32 = 20;
const PANEL_BG: [u8; 4] = (0xFF202020u32).to_le_bytes();
const TEXT_COLOR_NORMAL: [u8; 4] = (0xFFDDDDDDu32).to_le_bytes();
const TEXT_COLOR_KEY: [u8; 4] = (0xFFFFCC00u32).to_le_bytes();
const TITLE_COLOR: [u8; 4] = (0xFFFFFFFFu32).to_le_bytes();

struct HelpLine {
    key: &'static str,
    desc: &'static str,
    color: Option<u32>,
}

const COLOR_BOX_SIZE: u32 = 10;
const COLOR_BOX_GAP: u32 = GLYPH_W;
const BORDER_COLOR: [u8; 4] = (0xFF666666u32).to_le_bytes();

const HELP_TITLE: &str = "Waydoodle - Keyboard Shortcuts";

static HELP_LINES: LazyLock<Vec<HelpLine>> = LazyLock::new(|| {
    ALL_KEYS
        .iter()
        .map(|info| HelpLine {
            key: info.key_label,
            desc: info.desc,
            color: info.swatch(),
        })
        .collect()
});

pub(crate) fn render_help(canvas: &mut Canvas) -> Rect {
    let font_regular = &*FONT_REGULAR;
    let font_bold = &*FONT_BOLD;
    let help_lines = &*HELP_LINES;

    let key_col_width = {
        let mut max_w = 0u32;
        for line in help_lines {
            max_w = max_w.max(Canvas::text_width(line.key));
        }
        max_w + COLOR_BOX_GAP
    };

    let desc_col_width = {
        let mut max_w = 0u32;
        for line in help_lines {
            max_w = max_w.max(Canvas::text_width(line.desc));
        }
        max_w
    };

    let color_col_width = COLOR_BOX_SIZE + COLOR_BOX_GAP;
    let content_width = key_col_width + color_col_width + desc_col_width;
    let title_width = Canvas::text_width(HELP_TITLE);
    let inner_width = content_width.max(title_width);

    let num_lines = help_lines.len() as u32;
    let inner_height = LINE_HEIGHT + LINE_HEIGHT / 2 + num_lines * LINE_HEIGHT;

    let panel_w = inner_width + PADDING * 2;
    let panel_h = inner_height + PADDING * 2;

    let panel_x = (canvas.width.saturating_sub(panel_w) / 2) as i32;
    let panel_y = (canvas.height.saturating_sub(panel_h) / 2) as i32;

    canvas.draw_rect(panel_x, panel_y, panel_w, panel_h, PANEL_BG);
    canvas.draw_border(panel_x, panel_y, panel_w, panel_h, BORDER_COLOR);

    let text_x = panel_x + PADDING as i32;
    let mut row_y = panel_y + PADDING as i32;

    let title_x = panel_x + (panel_w as i32 - title_width as i32) / 2;
    canvas.draw_text(font_bold, HELP_TITLE, title_x, row_y, TITLE_COLOR);
    row_y += LINE_HEIGHT as i32 + LINE_HEIGHT as i32 / 2;

    for line in help_lines {
        let key_offset =
            key_col_width as i32 - Canvas::text_width(line.key) as i32 - COLOR_BOX_GAP as i32;
        canvas.draw_text(
            font_bold,
            line.key,
            text_x + key_offset,
            row_y,
            TEXT_COLOR_KEY,
        );
        let desc_x = text_x + key_col_width as i32 + color_col_width as i32;

        if let Some(argb) = line.color {
            let box_x = text_x + key_col_width as i32;
            let box_y = row_y + GLYPH_H as i32 - COLOR_BOX_SIZE as i32;
            canvas.draw_rect(
                box_x,
                box_y,
                COLOR_BOX_SIZE,
                COLOR_BOX_SIZE,
                argb.to_le_bytes(),
            );
        }

        canvas.draw_text(font_regular, line.desc, desc_x, row_y, TEXT_COLOR_NORMAL);
        row_y += LINE_HEIGHT as i32;
    }

    Rect {
        x: panel_x.max(0),
        y: panel_y.max(0),
        width: (panel_w as i32).min(canvas.width as i32 - panel_x.max(0)),
        height: (panel_h as i32).min(canvas.height as i32 - panel_y.max(0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;

    fn make_canvas(buf: &mut Vec<u8>, w: u32, h: u32) -> Canvas<'_> {
        buf.resize((w * h * 4) as usize, 0);
        Canvas {
            buf,
            width: w,
            height: h,
        }
    }

    fn pixel_at(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let off = (y as usize * w as usize + x as usize) * 4;
        [buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]
    }

    #[test]
    fn render_help_returns_valid_centered_damage_rect() {
        let w = 800;
        let h = 600;
        let mut buf = Vec::new();
        let mut canvas = make_canvas(&mut buf, w, h);

        let damage = render_help(&mut canvas);

        // Damage rect must be fully within canvas bounds.
        assert!(damage.x >= 0);
        assert!(damage.y >= 0);
        assert!(damage.x + damage.width <= w as i32);
        assert!(damage.y + damage.height <= h as i32);

        // Panel should be smaller than the canvas and centered, so there
        // should be margins on all sides.
        assert!(damage.x > 0, "expected left margin, got x={}", damage.x);
        assert!(damage.y > 0, "expected top margin, got y={}", damage.y);
        assert!(damage.x + damage.width < w as i32, "expected right margin");
        assert!(
            damage.y + damage.height < h as i32,
            "expected bottom margin"
        );

        // Rough centering check: the left and right margins should be equal.
        let left_margin = damage.x;
        let right_margin = w as i32 - (damage.x + damage.width);
        assert!(
            (left_margin - right_margin).abs() <= 1,
            "panel not horizontally centered: left={left_margin}, right={right_margin}"
        );
        let top_margin = damage.y;
        let bottom_margin = h as i32 - (damage.y + damage.height);
        assert!(
            (top_margin - bottom_margin).abs() <= 1,
            "panel not vertically centered: top={top_margin}, bottom={bottom_margin}"
        );
    }

    #[test]
    fn render_help_draws_panel_background_pixels() {
        let w = 800;
        let h = 600;
        let mut buf = Vec::new();
        let mut canvas = make_canvas(&mut buf, w, h);

        let damage = render_help(&mut canvas);

        // A pixel in the interior of the damage rect should be non-transparent
        // (it should have the panel background or text drawn on it).
        let cx = (damage.x + damage.width / 2) as u32;
        let cy = (damage.y + damage.height / 2) as u32;
        let center = pixel_at(&buf, w, cx, cy);
        assert_ne!(
            center,
            [0, 0, 0, 0],
            "center of help panel should not be transparent"
        );

        // A pixel well outside the panel should still be transparent.
        assert_eq!(pixel_at(&buf, w, 0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn render_help_on_tiny_canvas_clamps_to_bounds() {
        // Canvas smaller than the help panel.
        let w = 50;
        let h = 30;
        let mut buf = Vec::new();
        let mut canvas = make_canvas(&mut buf, w, h);

        let damage = render_help(&mut canvas);

        // Damage rect must still be within canvas bounds.
        assert!(damage.x >= 0);
        assert!(damage.y >= 0);
        assert!(damage.width > 0);
        assert!(damage.height > 0);
        assert!(damage.x + damage.width <= w as i32);
        assert!(damage.y + damage.height <= h as i32);
    }

    #[test]
    fn render_help_has_border_pixels() {
        let w = 800;
        let h = 600;
        let mut buf = Vec::new();
        let mut canvas = make_canvas(&mut buf, w, h);

        let damage = render_help(&mut canvas);

        // The top-left corner of the damage rect should have the border color.
        let corner = pixel_at(&buf, w, damage.x as u32, damage.y as u32);
        assert_eq!(
            corner, BORDER_COLOR,
            "top-left corner should be the border color"
        );

        // The bottom-right corner of the panel should also be the border.
        let br_x = (damage.x + damage.width - 1) as u32;
        let br_y = (damage.y + damage.height - 1) as u32;
        let corner_br = pixel_at(&buf, w, br_x, br_y);
        assert_eq!(
            corner_br, BORDER_COLOR,
            "bottom-right corner should be the border color"
        );
    }
}
