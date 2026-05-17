use cairo::{
    BorrowError, Context, FontFace, FontSlant, FontWeight, Format, ImageSurface, ImageSurfaceData,
    RectangleInt,
};

use crate::{
    actions::{Action, MenuComponent, Op},
    canvas::Point,
    waydoodle::InputButton,
};

// An interactive button: hoverable and yields an Op when activated.
struct MenuButton {
    rect: RectangleInt,
    action: &'static Action,
}

// A single row in the context menu.
enum Row {
    // "Pen [R] [G] [B] ..."  — non-hoverable label followed by compact buttons.
    Category {
        label: &'static str,
        label_rect: RectangleInt,
        first_button_idx: usize,
        button_count: usize,
    },
    // "Erase    E"  — full-width button with description on the left and accel on the right.
    Item {
        button_idx: usize,
    },
}

impl Row {
    const PADDING_X: i32 = 5;
    const PADDING_Y: i32 = 5;
    const SWATCH_SIZE: i32 = 16;

    fn height(comp: &MenuComponent, ctx: &Context) -> Result<i32, cairo::Error> {
        let fe = ctx.font_extents()?;
        let font_height = (fe.ascent() + fe.descent()).ceil() as i32;
        Ok(match comp {
            MenuComponent::Category { .. } => {
                Self::PADDING_Y * 2 + font_height.max(Self::SWATCH_SIZE)
            }
            MenuComponent::Item(_) => Self::PADDING_Y * 2 + font_height,
        })
    }

    fn calc_extents(comp: &MenuComponent, ctx: &Context) -> Result<(i32, i32), cairo::Error> {
        match comp {
            MenuComponent::Category { name, items } => {
                let label_w =
                    Self::PADDING_X * 2 + ctx.text_extents(name)?.x_advance().ceil() as i32;
                let btns_w = (Self::PADDING_X * 2 + Self::SWATCH_SIZE) * items.len() as i32;
                Ok((label_w + btns_w, Self::height(comp, ctx)?))
            }
            MenuComponent::Item(action) => {
                let desc_w = ctx.text_extents(action.desc)?.x_advance().ceil() as i32;
                let accel_w = ctx.text_extents(action.accel_label)?.x_advance().ceil() as i32;
                Ok((
                    Self::PADDING_X
                        + Self::SWATCH_SIZE
                        + Self::PADDING_X
                        + desc_w
                        + Self::PADDING_X
                        + accel_w
                        + Self::PADDING_X,
                    Self::height(comp, ctx)?,
                ))
            }
        }
    }

    fn make_row(
        comp: &'static MenuComponent,
        ctx: &Context,
        menu_rect: &RectangleInt,
        row_y: i32,
        first_button_idx: usize,
    ) -> Result<(Self, Vec<MenuButton>, i32), cairo::Error> {
        let row_h = Self::height(comp, ctx)?;
        match comp {
            MenuComponent::Category { name, items } => {
                let label_text_w = ctx.text_extents(name)?.x_advance().ceil() as i32;
                let label_w = Self::PADDING_X * 2 + label_text_w;
                let label_rect = RectangleInt::new(menu_rect.x(), row_y, label_w, row_h);

                // justify buttons to the right of the label, with padding in between
                let total_btns_w = (Self::PADDING_X * 2 + Self::SWATCH_SIZE) * items.len() as i32;
                let mut btn_x = menu_rect.x() + menu_rect.width() - total_btns_w;
                let mut buttons = Vec::with_capacity(items.len());
                for action in *items {
                    let btn_w = Self::PADDING_X * 2 + Self::SWATCH_SIZE;
                    buttons.push(MenuButton {
                        rect: RectangleInt::new(btn_x, row_y, btn_w, row_h),
                        action,
                    });
                    btn_x += btn_w;
                }

                Ok((
                    Row::Category {
                        label: name,
                        label_rect,
                        first_button_idx,
                        button_count: items.len(),
                    },
                    buttons,
                    row_h,
                ))
            }
            MenuComponent::Item(action) => {
                let buttons = vec![MenuButton {
                    rect: RectangleInt::new(menu_rect.x(), row_y, menu_rect.width(), row_h),
                    action,
                }];
                Ok((
                    Row::Item {
                        button_idx: first_button_idx,
                    },
                    buttons,
                    row_h,
                ))
            }
        }
    }

    fn render(
        &self,
        ctx: &Context,
        buttons: &[MenuButton],
        hover: Option<usize>,
    ) -> Result<(), cairo::Error> {
        let fe = ctx.font_extents()?;
        match self {
            Self::Category {
                label,
                label_rect,
                first_button_idx,
                button_count,
            } => {
                // Hover highlight on the hovered button (if any) in this category.
                for (bi, btn) in buttons
                    .iter()
                    .enumerate()
                    .skip(*first_button_idx)
                    .take(*button_count)
                {
                    if hover == Some(bi) {
                        ctx.set_source_rgb(0.2, 0.2, 0.2);
                        ctx.rectangle(
                            btn.rect.x() as f64,
                            btn.rect.y() as f64,
                            btn.rect.width() as f64,
                            btn.rect.height() as f64,
                        );
                        ctx.fill()?;
                    }
                }

                let baseline_y = label_rect.y() as f64 + Self::PADDING_Y as f64 + fe.ascent();

                // Category label (non-interactive).
                ctx.set_source_rgb(0.9, 0.9, 0.9);
                ctx.move_to(label_rect.x() as f64 + Self::PADDING_X as f64, baseline_y);
                ctx.show_text(label)?;

                // Each button: swatch + accel label.
                for btn in buttons.iter().skip(*first_button_idx).take(*button_count) {
                    let br = btn.rect;
                    let action = btn.action;

                    let swatch_x = br.x() as f64 + Self::PADDING_X as f64;
                    let swatch_y =
                        br.y() as f64 + (br.height() as f64 - Self::SWATCH_SIZE as f64) / 2.0;

                    let color = action.swatch().unwrap();
                    if color.a > 0 {
                        ctx.set_source_rgb(
                            color.r as f64 / 255.0,
                            color.g as f64 / 255.0,
                            color.b as f64 / 255.0,
                        );
                    } else {
                        // transparent swatch: draw a checkerboard pattern
                        let checkerboard = Self::checkerboard()?;
                        ctx.set_source_surface(&checkerboard, swatch_x, swatch_y)?;
                    }
                    ctx.rectangle(
                        swatch_x,
                        swatch_y,
                        Self::SWATCH_SIZE as f64,
                        Self::SWATCH_SIZE as f64,
                    );
                    ctx.fill()?;

                    // label color should have good contrast against the swatch
                    if color.luma() < 0.5 {
                        ctx.set_source_rgb(0.9, 0.9, 0.9);
                    } else {
                        ctx.set_source_rgb(0.1, 0.1, 0.1);
                    }

                    // center the accel label under the swatch
                    let extents = ctx.text_extents(action.accel_label)?;
                    ctx.move_to(
                        swatch_x + (Self::SWATCH_SIZE as f64 - extents.x_advance()) / 2.0,
                        baseline_y,
                    );
                    ctx.show_text(action.accel_label)?;
                }
            }
            Self::Item { button_idx } => {
                let btn = &buttons[*button_idx];

                // Hover highlight for this row.
                if hover == Some(*button_idx) {
                    ctx.set_source_rgb(0.2, 0.2, 0.2);
                    ctx.rectangle(
                        btn.rect.x() as f64,
                        btn.rect.y() as f64,
                        btn.rect.width() as f64,
                        btn.rect.height() as f64,
                    );
                    ctx.fill()?;
                }

                // Description text.
                let baseline_y = btn.rect.y() as f64 + Self::PADDING_Y as f64 + fe.ascent();
                ctx.set_source_rgb(0.9, 0.9, 0.9);
                ctx.move_to(btn.rect.x() as f64 + Self::PADDING_X as f64, baseline_y);
                ctx.show_text(btn.action.desc)?;

                // Accel label, right-aligned.
                ctx.set_source_rgb(0.7, 0.7, 0.7);
                let accel_adv = ctx.text_extents(btn.action.accel_label)?.x_advance();
                ctx.move_to(
                    btn.rect.x() as f64 + btn.rect.width() as f64
                        - Self::PADDING_X as f64
                        - accel_adv,
                    baseline_y,
                );
                ctx.show_text(btn.action.accel_label)?;
            }
        }
        Ok(())
    }

    fn checkerboard() -> Result<ImageSurface, cairo::Error> {
        let checkerboard =
            ImageSurface::create(Format::ARgb32, Self::SWATCH_SIZE, Self::SWATCH_SIZE)?;
        {
            let cb_ctx = Context::new(&checkerboard)?;
            cb_ctx.set_source_rgb(0.8, 0.8, 0.8);
            cb_ctx.paint()?;
            cb_ctx.set_source_rgb(0.6, 0.6, 0.6);
            for y in (0..Self::SWATCH_SIZE).step_by(4) {
                for x in (0..Self::SWATCH_SIZE).step_by(4) {
                    if (x + y) % 8 == 0 {
                        cb_ctx.rectangle(x as f64, y as f64, 4.0, 4.0);
                    }
                }
            }
            cb_ctx.fill()?;
        }
        Ok(checkerboard)
    }
}

pub struct ContextMenu {
    hover: Option<usize>, // flat index into `buttons`
    in_menu: bool,        // pointer is within the menu bounding rect (but not necessarily a button)
    rows: Vec<Row>,
    buttons: Vec<MenuButton>,
    rect: RectangleInt,
}

impl ContextMenu {
    pub fn new(
        pos: Point,
        menu: &'static [MenuComponent],
        screen_width: i32,
        screen_height: i32,
    ) -> Result<Self, cairo::Error> {
        let dummy = UI::dummy_surface()?;
        let ctx = UI::make_ctx(&dummy)?;

        // First pass: compute the menu size
        let mut menu_w = 0;
        let mut menu_h = 0;
        for comp in menu {
            let (w, h) = Row::calc_extents(comp, &ctx)?;
            menu_w = menu_w.max(w);
            menu_h += h;
        }

        let (origin_x, origin_y) =
            Self::calc_origin(pos, menu_w, menu_h, screen_width, screen_height);
        let menu_rect = RectangleInt::new(origin_x, origin_y, menu_w, menu_h);

        // Second pass: build rows and buttons with absolute screen positions.
        let mut rows = Vec::with_capacity(menu.len());
        let mut buttons: Vec<MenuButton> = Vec::new();
        {
            let mut row_y = origin_y;
            for comp in menu {
                let first_button_idx = buttons.len();
                let (row, row_buttons, row_h) =
                    Row::make_row(comp, &ctx, &menu_rect, row_y, first_button_idx)?;
                rows.push(row);
                buttons.extend(row_buttons);
                row_y += row_h;
            }
        }

        Ok(Self {
            hover: None,
            in_menu: false,
            rows,
            buttons,
            rect: menu_rect,
        })
    }

    /// Compute the menu origin (top-left corner) clamped to the screen.
    /// By default the menu opens below the cursor; if there isn't enough space it opens above.
    fn calc_origin(
        pos: Point,
        menu_w: i32,
        menu_h: i32,
        screen_w: i32,
        screen_h: i32,
    ) -> (i32, i32) {
        let draw_above = pos.y as i32 + menu_h > screen_h;
        // shift some px to avoid overlapping the cursor
        let mut y = if draw_above {
            pos.y as i32 - menu_h - Row::PADDING_Y
        } else {
            pos.y as i32 + Row::PADDING_Y
        };
        if y < 0 {
            y = 0;
        }
        let mut x = pos.x as i32 - menu_w / 2;
        if x + menu_w > screen_w {
            x = screen_w - menu_w;
        }
        if x < 0 {
            x = 0;
        }
        (x, y)
    }

    pub fn render(&self, ctx: &Context) -> Result<(), cairo::Error> {
        // Full menu background.
        ctx.set_source_rgb(0.1, 0.1, 0.1);
        ctx.rectangle(
            self.rect.x() as f64,
            self.rect.y() as f64,
            self.rect.width() as f64,
            self.rect.height() as f64,
        );
        ctx.fill()?;

        for row in &self.rows {
            row.render(ctx, &self.buttons, self.hover)?;
        }
        Ok(())
    }

    /// Update the hovered button based on the pointer position.
    /// Returns true if the hover state changed (including entering/leaving the menu area).
    pub fn update_hover(&mut self, pos: Point) -> bool {
        let new_hover = self
            .buttons
            .iter()
            .position(|btn| rect_contains(btn.rect, pos.x as i32, pos.y as i32));
        let new_in_menu = rect_contains(self.rect, pos.x as i32, pos.y as i32);
        if new_hover != self.hover || new_in_menu != self.in_menu {
            self.hover = new_hover;
            self.in_menu = new_in_menu;
            true
        } else {
            false
        }
    }
}

fn rect_contains(rect: RectangleInt, x: i32, y: i32) -> bool {
    x >= rect.x() && x < rect.x() + rect.width() && y >= rect.y() && y < rect.y() + rect.height()
}

pub struct UI {
    surface: ImageSurface,
    context_menu: Option<ContextMenu>,
    last_pointer_pos: Option<Point>,
    menu: &'static [MenuComponent],
}

impl UI {
    const FONT_SIZE: f64 = 14.0;
    const FONT_FAMILY: &'static str = ""; // use the default font
    const FONT_SLANT: FontSlant = FontSlant::Normal;
    const FONT_WEIGHT: FontWeight = FontWeight::Normal;

    fn dummy_surface() -> Result<ImageSurface, cairo::Error> {
        ImageSurface::create(Format::ARgb32, 1, 1)
    }

    fn make_ctx(surface: &ImageSurface) -> Result<Context, cairo::Error> {
        let font_face =
            FontFace::toy_create(Self::FONT_FAMILY, Self::FONT_SLANT, Self::FONT_WEIGHT)?;
        let ctx = Context::new(surface)?;
        ctx.set_font_face(&font_face);
        ctx.set_font_size(Self::FONT_SIZE);
        Ok(ctx)
    }

    pub fn new(
        width: i32,
        height: i32,
        menu: &'static [MenuComponent],
    ) -> Result<Self, cairo::Error> {
        Ok(Self {
            surface: ImageSurface::create(Format::ARgb32, width, height)?,
            context_menu: None,
            last_pointer_pos: None,
            menu,
        })
    }

    pub fn surface_data(&'_ mut self) -> Result<ImageSurfaceData<'_>, BorrowError> {
        self.surface.data()
    }

    pub fn on_pointer_button_pressed(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> Result<(Option<Op>, bool), cairo::Error> {
        let Some(mut menu) = self.context_menu.take() else {
            if btn == InputButton::Secondary {
                // Open the context menu.
                self.context_menu = Some(ContextMenu::new(
                    pos,
                    self.menu,
                    self.surface.width(),
                    self.surface.height(),
                )?);
                self.render()?;
                return Ok((None, true));
            }
            // No menu open and not a right-click — nothing to do.
            return Ok((None, false));
        };
        // Menu was open: close it and trigger the action under the cursor (if any).
        menu.update_hover(pos);
        self.render()?;
        Ok((menu.hover.map(|idx| menu.buttons[idx].action.op), true))
    }

    pub fn on_pointer_button_released(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> Result<(Option<Op>, bool), cairo::Error> {
        let op = {
            let Some(menu) = self.context_menu.as_mut() else {
                // No menu open — nothing to do.
                return Ok((None, false));
            };
            let _ = menu.update_hover(pos);
            if btn == InputButton::Secondary {
                // Right-click release: trigger the action under the cursor (if any).
                menu.hover.map(|idx| menu.buttons[idx].action.op)
            } else {
                // Other button release: close menu without triggering an action.
                None
            }
        };

        // Close the menu if an action was triggered
        if op.is_some() {
            self.context_menu = None;
        }
        self.render()?;
        Ok((op, true))
    }

    pub fn on_pointer_motion(&mut self, pos: Point) -> Result<bool, cairo::Error> {
        self.last_pointer_pos = Some(pos);
        if let Some(menu) = &mut self.context_menu {
            if menu.update_hover(pos) {
                self.render()?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    fn render(&mut self) -> Result<(), cairo::Error> {
        let ctx = UI::make_ctx(&self.surface)?;
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        ctx.set_operator(cairo::Operator::Source);
        ctx.paint()?;
        if let Some(menu) = &self.context_menu {
            menu.render(&ctx)?;
        }
        Ok(())
    }

    pub fn toggle_context_menu(&mut self) -> Result<(), cairo::Error> {
        if self.context_menu.is_some() {
            self.context_menu = None;
        } else {
            let pos = self.last_pointer_pos.unwrap_or(Point { x: 0.0, y: 0.0 });
            self.context_menu = Some(ContextMenu::new(
                pos,
                self.menu,
                self.surface.width(),
                self.surface.height(),
            )?);
        }
        self.render()?;
        Ok(())
    }

    pub(crate) fn is_context_menu_open(&self) -> bool {
        self.context_menu.is_some()
    }

    pub(crate) fn context_menu_rect(&self) -> Option<RectangleInt> {
        self.context_menu.as_ref().map(|menu| menu.rect)
    }
}
