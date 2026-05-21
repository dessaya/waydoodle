use cairo::{
    BorrowError, Context, FontFace, FontSlant, FontWeight, Format, ImageSurface, ImageSurfaceData,
    RectangleInt,
};
use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::{
    actions::{Action, FocusDirection, GLOBAL_ACCELS, NO_MENU_ACCELS},
    canvas::{Color, Point},
    waydoodle::{InputButton, Result, Tool},
};

enum MenuComponent {
    Category {
        name: &'static str,
        label_rect: RectangleInt,
        items: Vec<SwatchMenuItem>,
    },
    Item(RowMenuItem),
}

struct SwatchMenuItem {
    pub swatch: Color,
    pub btn: MenuButton,
}

struct RowMenuItem {
    pub label: &'static str,
    pub btn: MenuButton,
}

struct MenuButton {
    pub id: usize,
    pub action: Action,
    pub rect: RectangleInt,
    pub accel: &'static str,
}

impl MenuButton {
    fn hit(&self, x: i32, y: i32) -> bool {
        x >= self.rect.x()
            && x < self.rect.x() + self.rect.width()
            && y >= self.rect.y()
            && y < self.rect.y() + self.rect.height()
    }

    fn dy(&self, btn: &MenuButton) -> i32 {
        btn.rect.y() - self.rect.y()
    }

    fn dx(&self, btn: &MenuButton) -> i32 {
        btn.rect.x() - self.rect.x()
    }
}

fn build_menu() -> Vec<MenuComponent> {
    let mut id = 0;
    let pen_items = build_pen_menu_items(id);
    id += pen_items.len();
    let eraser_item = build_row_menu_item(id, "Eraser", Action::SetTool(Tool::Eraser));
    id += 1;
    let background_items = build_background_menu_items(id);
    id += background_items.len();
    let clear_item = build_row_menu_item(id, "Clear screen", Action::Clear);
    id += 1;
    let undo_item = build_row_menu_item(id, "Undo", Action::Undo);
    id += 1;
    let hide_overlay_item = build_row_menu_item(id, "Hide overlay", Action::HideOverlay);

    vec![
        MenuComponent::Category {
            name: "Pen",
            items: pen_items,
            label_rect: RectangleInt::new(0, 0, 0, 0),
        },
        MenuComponent::Item(eraser_item),
        MenuComponent::Category {
            name: "Background",
            items: background_items,
            label_rect: RectangleInt::new(0, 0, 0, 0),
        },
        MenuComponent::Item(clear_item),
        MenuComponent::Item(undo_item),
        MenuComponent::Item(hide_overlay_item),
    ]
}

fn build_pen_menu_items(start_id: usize) -> Vec<SwatchMenuItem> {
    GLOBAL_ACCELS
        .iter()
        .filter(|(_, action)| matches!(action, Action::SetTool(Tool::Pen(_))))
        .enumerate()
        .map(move |(i, (keysym, action))| match action {
            Action::SetTool(Tool::Pen(color)) => SwatchMenuItem {
                swatch: *color,
                btn: MenuButton {
                    id: start_id + i,
                    action: *action,
                    accel: accel_label(*keysym),
                    rect: RectangleInt::new(0, 0, 0, 0),
                },
            },
            _ => unreachable!(),
        })
        .collect()
}

fn build_background_menu_items(start_id: usize) -> Vec<SwatchMenuItem> {
    GLOBAL_ACCELS
        .iter()
        .filter(|(_, action)| matches!(action, Action::SetBackground(_)))
        .enumerate()
        .map(move |(i, (keysym, action))| match action {
            Action::SetBackground(color) => SwatchMenuItem {
                swatch: *color,
                btn: MenuButton {
                    id: start_id + i,
                    action: *action,
                    accel: accel_label(*keysym),
                    rect: RectangleInt::new(0, 0, 0, 0),
                },
            },
            _ => unreachable!(),
        })
        .collect()
}

fn build_row_menu_item(id: usize, label: &'static str, action: Action) -> RowMenuItem {
    let accel = GLOBAL_ACCELS
        .iter()
        .chain(NO_MENU_ACCELS.iter())
        .find_map(|(keysym, a)| {
            if *a == action {
                Some(accel_label(*keysym))
            } else {
                None
            }
        })
        .unwrap_or("");
    RowMenuItem {
        label,
        btn: MenuButton {
            id,
            action,
            accel,
            rect: RectangleInt::new(0, 0, 0, 0),
        },
    }
}

fn accel_label(keysym: Keysym) -> &'static str {
    match keysym {
        Keysym::space => "Space",
        Keysym::Escape => "Esc",
        Keysym::r => "R",
        Keysym::g => "G",
        Keysym::b => "B",
        Keysym::y => "Y",
        Keysym::m => "M",
        Keysym::n => "N",
        Keysym::e => "E",
        Keysym::period => ".",
        Keysym::comma => ",",
        Keysym::slash => "/",
        Keysym::c => "C",
        Keysym::u => "U",
        _ => keysym.name().unwrap_or("?"),
    }
}

impl MenuComponent {
    const PADDING_X: i32 = 5;
    const PADDING_Y: i32 = 5;
    const SWATCH_SIZE: i32 = 16;

    fn height(&self, ctx: &Context) -> Result<i32> {
        let fe = ctx.font_extents()?;
        let font_height = (fe.ascent() + fe.descent()).ceil() as i32;
        Ok(match self {
            MenuComponent::Category { .. } => {
                Self::PADDING_Y * 2 + font_height.max(Self::SWATCH_SIZE)
            }
            MenuComponent::Item(_) => Self::PADDING_Y * 2 + font_height,
        })
    }

    fn calc_extents(&self, ctx: &Context) -> Result<(i32, i32)> {
        match self {
            MenuComponent::Category { name, items, .. } => {
                let label_w =
                    Self::PADDING_X * 2 + ctx.text_extents(name)?.x_advance().ceil() as i32;
                let btns_w = (Self::PADDING_X * 2 + Self::SWATCH_SIZE) * items.len() as i32;
                Ok((label_w + btns_w, self.height(ctx)?))
            }
            MenuComponent::Item(item) => {
                let desc_w = ctx.text_extents(item.label)?.x_advance().ceil() as i32;
                let accel_w = ctx.text_extents(item.btn.accel)?.x_advance().ceil() as i32;
                Ok((
                    Self::PADDING_X
                        + Self::SWATCH_SIZE
                        + Self::PADDING_X
                        + desc_w
                        + Self::PADDING_X
                        + accel_w
                        + Self::PADDING_X,
                    self.height(ctx)?,
                ))
            }
        }
    }

    fn layout_row(&mut self, ctx: &Context, menu_rect: &RectangleInt, row_y: i32) -> Result<i32> {
        let row_h = self.height(ctx)?;
        match self {
            MenuComponent::Category {
                name,
                items,
                label_rect,
            } => {
                let label_text_w = ctx.text_extents(name)?.x_advance().ceil() as i32;
                let label_w = Self::PADDING_X * 2 + label_text_w;
                *label_rect = RectangleInt::new(menu_rect.x(), row_y, label_w, row_h);

                // justify buttons to the right of the label, with padding in between
                let total_btns_w = (Self::PADDING_X * 2 + Self::SWATCH_SIZE) * items.len() as i32;
                let mut btn_x = menu_rect.x() + menu_rect.width() - total_btns_w;
                for item in items {
                    let btn_w = Self::PADDING_X * 2 + Self::SWATCH_SIZE;
                    item.btn.rect = RectangleInt::new(btn_x, row_y, btn_w, row_h);
                    btn_x += btn_w;
                }
            }
            MenuComponent::Item(item) => {
                item.btn.rect = RectangleInt::new(menu_rect.x(), row_y, menu_rect.width(), row_h);
            }
        };
        Ok(row_h)
    }

    fn render(&self, ctx: &Context, hover: Option<usize>) -> Result<()> {
        let fe = ctx.font_extents()?;
        match self {
            MenuComponent::Category {
                name,
                label_rect,
                items,
            } => {
                // Hover highlight on the hovered button (if any) in this category.
                if let Some(hover) = hover
                    && let Some(item) = items.iter().find(|item| item.btn.id == hover)
                {
                    ctx.set_source_rgb(0.2, 0.2, 0.2);
                    ctx.rectangle(
                        item.btn.rect.x() as f64,
                        item.btn.rect.y() as f64,
                        item.btn.rect.width() as f64,
                        item.btn.rect.height() as f64,
                    );
                    ctx.fill()?;
                }

                let baseline_y = label_rect.y() as f64 + Self::PADDING_Y as f64 + fe.ascent();

                // Category label (non-interactive).
                ctx.set_source_rgb(0.9, 0.9, 0.9);
                ctx.move_to(label_rect.x() as f64 + Self::PADDING_X as f64, baseline_y);
                ctx.show_text(name)?;

                // Each button: swatch + accel label.
                for item in items {
                    let br = item.btn.rect;

                    let swatch_x = br.x() as f64 + Self::PADDING_X as f64;
                    let swatch_y =
                        br.y() as f64 + (br.height() as f64 - Self::SWATCH_SIZE as f64) / 2.0;

                    let color = item.swatch;
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
                    let extents = ctx.text_extents(item.btn.accel)?;
                    ctx.move_to(
                        swatch_x + (Self::SWATCH_SIZE as f64 - extents.x_advance()) / 2.0,
                        baseline_y,
                    );
                    ctx.show_text(item.btn.accel)?;
                }
            }
            MenuComponent::Item(item) => {
                // Hover highlight for this row.
                if hover == Some(item.btn.id) {
                    ctx.set_source_rgb(0.2, 0.2, 0.2);
                    ctx.rectangle(
                        item.btn.rect.x() as f64,
                        item.btn.rect.y() as f64,
                        item.btn.rect.width() as f64,
                        item.btn.rect.height() as f64,
                    );
                    ctx.fill()?;
                }

                // Description text.
                let baseline_y = item.btn.rect.y() as f64 + Self::PADDING_Y as f64 + fe.ascent();
                ctx.set_source_rgb(0.9, 0.9, 0.9);
                ctx.move_to(
                    item.btn.rect.x() as f64 + Self::PADDING_X as f64,
                    baseline_y,
                );
                ctx.show_text(item.label)?;

                // Accel label, right-aligned.
                ctx.set_source_rgb(0.7, 0.7, 0.7);
                let accel_adv = ctx.text_extents(item.btn.accel)?.x_advance();
                ctx.move_to(
                    item.btn.rect.x() as f64 + item.btn.rect.width() as f64
                        - Self::PADDING_X as f64
                        - accel_adv,
                    baseline_y,
                );
                ctx.show_text(item.btn.accel)?;
            }
        }
        Ok(())
    }

    fn checkerboard() -> Result<ImageSurface> {
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
    components: Vec<MenuComponent>,
    hover: Option<usize>,
    rect: RectangleInt,
}

impl ContextMenu {
    pub fn new(pos: Point, screen_width: i32, screen_height: i32) -> Result<Self> {
        let dummy = UI::dummy_surface()?;
        let ctx = UI::make_ctx(&dummy)?;

        let mut menu = build_menu();

        // First pass: compute the menu size
        let mut menu_w = 0;
        let mut menu_h = 0;
        for comp in menu.iter() {
            let (w, h) = comp.calc_extents(&ctx)?;
            menu_w = menu_w.max(w);
            menu_h += h;
        }

        let (origin_x, origin_y) =
            Self::calc_origin(pos, menu_w, menu_h, screen_width, screen_height);
        let menu_rect = RectangleInt::new(origin_x, origin_y, menu_w, menu_h);

        // Second pass: layout elements
        {
            let mut row_y = origin_y;
            for comp in menu.iter_mut() {
                row_y += comp.layout_row(&ctx, &menu_rect, row_y)?;
            }
        }

        Ok(Self {
            components: menu,
            hover: None,
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
            pos.y as i32 - menu_h - MenuComponent::PADDING_Y
        } else {
            pos.y as i32 + MenuComponent::PADDING_Y
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

    pub fn render(&self, ctx: &Context) -> Result<()> {
        // Full menu background.
        ctx.set_source_rgb(0.1, 0.1, 0.1);
        ctx.rectangle(
            self.rect.x() as f64,
            self.rect.y() as f64,
            self.rect.width() as f64,
            self.rect.height() as f64,
        );
        ctx.fill()?;

        for comp in self.components.iter() {
            comp.render(ctx, self.hover)?;
        }
        Ok(())
    }

    fn buttons(&self) -> impl Iterator<Item = &MenuButton> {
        self.components
            .iter()
            .flat_map(|comp| -> Box<dyn Iterator<Item = &MenuButton>> {
                match comp {
                    MenuComponent::Category { items, .. } => {
                        Box::new(items.iter().map(|item| &item.btn))
                    }
                    MenuComponent::Item(item) => Box::new(std::iter::once(&item.btn)),
                }
            })
    }

    /// Update the hovered button based on the pointer position.
    /// Returns true if the hover state changed (including entering/leaving the menu area).
    pub fn update_hover(&mut self, pos: Point) -> bool {
        let new_hover = self
            .buttons()
            .find(|btn| btn.hit(pos.x as i32, pos.y as i32))
            .map(|btn| btn.id);
        if new_hover != self.hover {
            self.hover = new_hover;
            true
        } else {
            false
        }
    }

    fn selected_button(&self) -> Option<&MenuButton> {
        self.buttons().find(|btn| Some(btn.id) == self.hover)
    }

    fn selected_action(&self) -> Option<Action> {
        self.selected_button().map(|btn| btn.action)
    }
}

pub struct UI {
    surface: ImageSurface,
    context_menu: Option<ContextMenu>,
    last_pointer_pos: Option<Point>,
}

impl UI {
    const FONT_SIZE: f64 = 14.0;
    const FONT_FAMILY: &str = ""; // use the default font
    const FONT_SLANT: FontSlant = FontSlant::Normal;
    const FONT_WEIGHT: FontWeight = FontWeight::Normal;

    fn dummy_surface() -> Result<ImageSurface> {
        ImageSurface::create(Format::ARgb32, 1, 1)
    }

    fn make_ctx(surface: &ImageSurface) -> Result<Context> {
        let font_face =
            FontFace::toy_create(Self::FONT_FAMILY, Self::FONT_SLANT, Self::FONT_WEIGHT)?;
        let ctx = Context::new(surface)?;
        ctx.set_font_face(&font_face);
        ctx.set_font_size(Self::FONT_SIZE);
        Ok(ctx)
    }

    pub fn new(width: i32, height: i32) -> Result<Self> {
        Ok(Self {
            surface: ImageSurface::create(Format::ARgb32, width, height)?,
            context_menu: None,
            last_pointer_pos: None,
        })
    }

    pub fn surface_data(&'_ mut self) -> std::result::Result<ImageSurfaceData<'_>, BorrowError> {
        self.surface.data()
    }

    pub fn on_pointer_button_pressed(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> Result<(Option<Action>, bool)> {
        let Some(mut menu) = self.context_menu.take() else {
            if btn == InputButton::Secondary {
                // Open the context menu.
                self.context_menu = Some(ContextMenu::new(
                    pos,
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
        Ok((menu.selected_action(), true))
    }

    pub fn on_pointer_button_released(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> Result<(Option<Action>, bool)> {
        let action = {
            let Some(menu) = self.context_menu.as_mut() else {
                // No menu open — nothing to do.
                return Ok((None, false));
            };
            let _ = menu.update_hover(pos);
            if btn == InputButton::Secondary {
                // Right-click release: trigger the action under the cursor (if any).
                menu.selected_action()
            } else {
                // Other button release: close menu without triggering an action.
                None
            }
        };

        // Close the menu if an action was triggered
        if action.is_some() {
            self.context_menu = None;
        }
        self.render()?;
        Ok((action, true))
    }

    pub fn on_pointer_motion(&mut self, pos: Point) -> Result<bool> {
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

    fn render(&mut self) -> Result<()> {
        let ctx = UI::make_ctx(&self.surface)?;
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        ctx.set_operator(cairo::Operator::Source);
        ctx.paint()?;
        if let Some(menu) = &self.context_menu {
            menu.render(&ctx)?;
        }
        Ok(())
    }

    pub fn open_context_menu(&mut self) -> Result<()> {
        if self.context_menu.is_none() {
            let pos = self.last_pointer_pos.unwrap_or(Point { x: 0.0, y: 0.0 });
            self.context_menu = Some(ContextMenu::new(
                pos,
                self.surface.width(),
                self.surface.height(),
            )?);
            self.render()?;
        }
        Ok(())
    }

    pub(crate) fn focus_menu_item(&mut self, direction: FocusDirection) -> Result<()> {
        let Some(menu) = self.context_menu.as_mut() else {
            // No menu open — nothing to focus.
            return Ok(());
        };
        let current = menu.selected_button();
        // find the next one in the given direction, wrapping around if necessary
        let next = match current {
            Some(current) => {
                let btns = menu.buttons().filter(|btn| btn.id != current.id);
                match direction {
                    FocusDirection::Up => {
                        // find the button with a rect that is right above
                        btns.filter(|btn| current.dy(btn) < 0)
                            .min_by_key(|btn| (-current.dy(btn), current.dx(btn).abs()))
                            .or_else(|| {
                                // if there isn't one, wrap around to the bottom-most button
                                menu.buttons()
                                    .max_by_key(|btn| (btn.rect.y(), btn.rect.x()))
                            })
                    }
                    FocusDirection::Down => {
                        // find the button with a rect that is right below
                        btns.filter(|btn| current.dy(btn) > 0)
                            .min_by_key(|btn| (current.dy(btn), current.dx(btn).abs()))
                            .or_else(|| {
                                // if there isn't one, wrap around to the top-most button
                                menu.buttons()
                                    .min_by_key(|btn| (btn.rect.y(), btn.rect.x()))
                            })
                    }
                    FocusDirection::Left => {
                        // find the button with a rect that is right to the left
                        btns.filter(|btn| current.dx(btn) < 0)
                            .min_by_key(|btn| (-current.dx(btn), current.dy(btn).abs()))
                            .or_else(|| {
                                // if there isn't one, wrap around to the right-most button
                                menu.buttons()
                                    .max_by_key(|btn| (btn.rect.x(), btn.rect.y()))
                            })
                    }
                    FocusDirection::Right => {
                        // find the button with a rect that is right to the right
                        btns.filter(|btn| current.dx(btn) > 0)
                            .min_by_key(|btn| (current.dx(btn), current.dy(btn).abs()))
                            .or_else(|| {
                                // if there isn't one, wrap around to the left-most button
                                menu.buttons()
                                    .min_by_key(|btn| (btn.rect.x(), btn.rect.y()))
                            })
                    }
                }
            }
            None => match direction {
                FocusDirection::Up | FocusDirection::Left => menu.buttons().last(),
                FocusDirection::Down | FocusDirection::Right => menu.buttons().next(),
            },
        };
        if let Some(next) = next {
            menu.hover = Some(next.id);
        }
        self.render()?;
        Ok(())
    }

    pub fn close_context_menu(&mut self) -> Result<()> {
        if self.context_menu.is_some() {
            self.context_menu = None;
            self.render()?;
        }
        Ok(())
    }

    pub(crate) fn is_context_menu_open(&self) -> bool {
        self.context_menu.is_some()
    }

    pub(crate) fn context_menu_rect(&self) -> Option<RectangleInt> {
        self.context_menu.as_ref().map(|menu| menu.rect)
    }

    pub(crate) fn get_menu_selection(&self) -> Option<Action> {
        self.context_menu
            .as_ref()
            .and_then(|menu| menu.selected_action())
    }
}
