use std::io::BufReader;

use bdf_reader::Font;

use super::render::DirtyRect;

const FONT_REGULAR: &[u8] = include_bytes!("../../assets/Tamzen8x16r.bdf");
const FONT_BOLD: &[u8] = include_bytes!("../../assets/Tamzen8x16b.bdf");
const GLYPH_W: u32 = 8;
const GLYPH_H: u32 = 16;
const LINE_HEIGHT: u32 = GLYPH_H + 4;
const PADDING: u32 = 20;
const PANEL_BG: [u8; 4] = (0xE0202020u32).to_le_bytes();
const TEXT_COLOR_NORMAL: [u8; 4] = (0xFFDDDDDDu32).to_le_bytes();
const TEXT_COLOR_KEY: [u8; 4] = (0xFFFFCC00u32).to_le_bytes();
const TITLE_COLOR: [u8; 4] = (0xFFFFFFFFu32).to_le_bytes();
const BORDER_COLOR: [u8; 4] = (0xFF666666u32).to_le_bytes();

struct HelpLine {
    key: &'static str,
    desc: &'static str,
    color: Option<u32>,
}

const COLOR_BOX_SIZE: u32 = 10;
const COLOR_BOX_GAP: u32 = GLYPH_W;

const HELP_TITLE: &str = "Waydoodle - Keyboard Shortcuts";

const HELP_LINES: &[HelpLine] = &[
    HelpLine {
        key: "R",
        desc: "Red pen",
        color: Some(0xFFFF0000),
    },
    HelpLine {
        key: "G",
        desc: "Green pen",
        color: Some(0xFF00FF00),
    },
    HelpLine {
        key: "B",
        desc: "Blue pen",
        color: Some(0xFF0000FF),
    },
    HelpLine {
        key: "Y",
        desc: "Yellow pen",
        color: Some(0xFFFFFF00),
    },
    HelpLine {
        key: "M",
        desc: "Magenta pen",
        color: Some(0xFFFF00FF),
    },
    HelpLine {
        key: "N",
        desc: "Cyan pen",
        color: Some(0xFF00FFFF),
    },
    HelpLine {
        key: "E",
        desc: "Eraser",
        color: None,
    },
    HelpLine {
        key: "C",
        desc: "Clear screen",
        color: None,
    },
    HelpLine {
        key: "Esc",
        desc: "Hide overlay",
        color: None,
    },
    HelpLine {
        key: "F1",
        desc: "Toggle this help",
        color: None,
    },
];

fn set_pixel(canvas: &mut [u8], buf_width: u32, buf_height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= buf_width as i32 || y >= buf_height as i32 {
        return;
    }
    let offset = (y as usize * buf_width as usize + x as usize) * 4;
    if offset + 4 <= canvas.len() {
        canvas[offset..offset + 4].copy_from_slice(&color);
    }
}

fn fill_rect(
    canvas: &mut [u8],
    buf_width: u32,
    buf_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    pixel: [u8; 4],
) {
    let x0 = x.max(0) as u32;
    let y0 = y.max(0) as u32;
    let x1 = ((x + w as i32) as u32).min(buf_width);
    let y1 = ((y + h as i32) as u32).min(buf_height);
    for row in y0..y1 {
        for col in x0..x1 {
            let offset = (row as usize * buf_width as usize + col as usize) * 4;
            if offset + 4 <= canvas.len() {
                canvas[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
    }
}

fn draw_text(
    canvas: &mut [u8],
    buf_width: u32,
    buf_height: u32,
    font: &Font,
    text: &str,
    x: i32,
    y: i32,
    color: [u8; 4],
) {
    let font_bb = font.bounding_box();
    let mut cursor_x = x;
    for ch in text.chars() {
        if let Some(glyph) = font.glyph(ch) {
            let bb = glyph.bounding_box();
            let bitmap = glyph.bitmap();
            for row in 0..bb.height as usize {
                for col in 0..bb.width as usize {
                    if bitmap.get(col, row).unwrap_or(false) {
                        let px = cursor_x + bb.offset_x + col as i32;
                        let py = y
                            + (font_bb.height as i32 - bb.offset_y - bb.height as i32)
                            + row as i32;
                        set_pixel(canvas, buf_width, buf_height, px, py, color);
                    }
                }
            }
            let advance = glyph
                .dwidth()
                .map(|(dx, _)| dx as i32)
                .unwrap_or(GLYPH_W as i32);
            cursor_x += advance;
        } else {
            cursor_x += GLYPH_W as i32;
        }
    }
}

fn text_width(text: &str) -> u32 {
    text.len() as u32 * GLYPH_W
}

fn draw_border(canvas: &mut [u8], buf_width: u32, buf_height: u32, x: i32, y: i32, w: u32, h: u32) {
    fill_rect(canvas, buf_width, buf_height, x, y, w, 1, BORDER_COLOR);
    fill_rect(
        canvas,
        buf_width,
        buf_height,
        x,
        y + h as i32 - 1,
        w,
        1,
        BORDER_COLOR,
    );
    fill_rect(canvas, buf_width, buf_height, x, y, 1, h, BORDER_COLOR);
    fill_rect(
        canvas,
        buf_width,
        buf_height,
        x + w as i32 - 1,
        y,
        1,
        h,
        BORDER_COLOR,
    );
}

pub(super) fn render_help(canvas: &mut [u8], buf_width: u32, buf_height: u32) -> DirtyRect {
    let font_regular =
        Font::read(BufReader::new(FONT_REGULAR)).expect("Failed to parse regular BDF font");
    let font_bold = Font::read(BufReader::new(FONT_BOLD)).expect("Failed to parse bold BDF font");

    let key_col_width = {
        let mut max_w = 0u32;
        for line in HELP_LINES {
            max_w = max_w.max(text_width(line.key));
        }
        max_w + COLOR_BOX_GAP
    };

    let desc_col_width = {
        let mut max_w = 0u32;
        for line in HELP_LINES {
            max_w = max_w.max(text_width(line.desc));
        }
        max_w
    };

    let color_col_width = COLOR_BOX_SIZE + COLOR_BOX_GAP;
    let content_width = key_col_width + color_col_width + desc_col_width;
    let title_width = text_width(HELP_TITLE);
    let inner_width = content_width.max(title_width);

    let num_lines = HELP_LINES.len() as u32;
    let inner_height = LINE_HEIGHT + LINE_HEIGHT / 2 + num_lines * LINE_HEIGHT;

    let panel_w = inner_width + PADDING * 2;
    let panel_h = inner_height + PADDING * 2;

    let panel_x = (buf_width.saturating_sub(panel_w) / 2) as i32;
    let panel_y = (buf_height.saturating_sub(panel_h) / 2) as i32;

    fill_rect(
        canvas, buf_width, buf_height, panel_x, panel_y, panel_w, panel_h, PANEL_BG,
    );
    draw_border(
        canvas, buf_width, buf_height, panel_x, panel_y, panel_w, panel_h,
    );

    let text_x = panel_x + PADDING as i32;
    let mut row_y = panel_y + PADDING as i32;

    let title_x = panel_x + (panel_w as i32 - title_width as i32) / 2;
    draw_text(
        canvas,
        buf_width,
        buf_height,
        &font_bold,
        HELP_TITLE,
        title_x,
        row_y,
        TITLE_COLOR,
    );
    row_y += LINE_HEIGHT as i32 + LINE_HEIGHT as i32 / 2;

    for line in HELP_LINES {
        let key_offset = key_col_width as i32 - text_width(line.key) as i32 - COLOR_BOX_GAP as i32;
        draw_text(
            canvas,
            buf_width,
            buf_height,
            &font_bold,
            line.key,
            text_x + key_offset,
            row_y,
            TEXT_COLOR_KEY,
        );
        let desc_x = text_x + key_col_width as i32 + color_col_width as i32;

        if let Some(argb) = line.color {
            let box_x = text_x + key_col_width as i32;
            let box_y = row_y + GLYPH_H as i32 - COLOR_BOX_SIZE as i32;
            fill_rect(
                canvas,
                buf_width,
                buf_height,
                box_x,
                box_y,
                COLOR_BOX_SIZE,
                COLOR_BOX_SIZE,
                argb.to_le_bytes(),
            );
        }

        draw_text(
            canvas,
            buf_width,
            buf_height,
            &font_regular,
            line.desc,
            desc_x,
            row_y,
            TEXT_COLOR_NORMAL,
        );
        row_y += LINE_HEIGHT as i32;
    }

    DirtyRect {
        x: panel_x.max(0),
        y: panel_y.max(0),
        width: (panel_w as i32).min(buf_width as i32 - panel_x.max(0)),
        height: (panel_h as i32).min(buf_height as i32 - panel_y.max(0)),
    }
}
