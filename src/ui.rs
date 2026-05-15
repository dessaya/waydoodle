use cairo::{
    BorrowError, Context, FontFace, FontSlant, FontWeight, Format, ImageSurface, ImageSurfaceData,
    RectangleInt,
};

use crate::{
    actions::{Action, Op},
    canvas::Point,
    waydoodle::InputButton,
};

pub struct ContextMenu {
    hover: Option<usize>, // index of the hovered menu item
    items: Vec<MenuItem>,
    rect: RectangleInt,
}

impl ContextMenu {
    const FONT_SIZE: f64 = 14.0;
    const FONT_FAMILY: &'static str = ""; // use the default font
    const FONT_SLANT: FontSlant = FontSlant::Normal;
    const FONT_WEIGHT: FontWeight = FontWeight::Normal;

    const PADDING_X: i32 = 10;
    const PADDING_Y: i32 = 5;

    const SWATCH_SIZE: i32 = 14;

    pub fn new(
        pos: Point,
        actions: &[Action],
        screen_width: i32,
        screen_height: i32,
    ) -> Result<Self, cairo::Error> {
        let items = Self::make_items(actions)?;
        let rect = Self::calc_rect(&items, pos, screen_width, screen_height);
        Ok(Self {
            hover: None,
            items,
            rect,
        })
    }

    pub fn make_items(actions: &[Action]) -> Result<Vec<MenuItem>, cairo::Error> {
        let font_face =
            FontFace::toy_create(Self::FONT_FAMILY, Self::FONT_SLANT, Self::FONT_WEIGHT)?;

        // to measure text extents, we need a Cairo context. We can create a
        // dummy surface and context for this purpose.
        let dummy_target = ImageSurface::create(Format::ARgb32, 1, 1)?;
        let dummy_ctx = Context::new(&dummy_target)?;
        dummy_ctx.set_font_face(&font_face);
        dummy_ctx.set_font_size(Self::FONT_SIZE);

        let desc_width = Self::max_width(
            &dummy_ctx,
            &actions.iter().map(|item| item.desc).collect::<Vec<_>>(),
        )?;

        let accel_width = Self::max_width(
            &dummy_ctx,
            &actions
                .iter()
                .map(|item| item.accel_label)
                .collect::<Vec<_>>(),
        )?;

        let mut items = Vec::new();
        for idx in 0..actions.len() {
            items.push(Self::make_menu_item(
                &dummy_ctx,
                &font_face,
                actions,
                idx,
                desc_width,
                accel_width,
            )?);
        }

        Ok(items)
    }

    fn make_menu_item(
        dummy_ctx: &Context,
        font_face: &FontFace,
        actions: &[Action],
        idx: usize,
        desc_width: i32,
        accel_width: i32,
    ) -> Result<MenuItem, cairo::Error> {
        // Each menu item consists of a swatch (if applicable), a description, and the
        // accel label. The layout is as follows:
        //
        //   PADDING_X swatch PADDING_X desc_width PADDING_X accel_label PADDING_X

        let item = &actions[idx];
        let desc_height = Self::text_height(dummy_ctx, item.desc)?;
        let accel_height = Self::text_height(dummy_ctx, item.accel_label)?;
        let total_width = Self::PADDING_X * 4 + Self::SWATCH_SIZE + desc_width + accel_width;
        let total_height =
            Self::PADDING_Y * 2 + desc_height.max(accel_height).max(Self::SWATCH_SIZE);
        Ok(MenuItem {
            idx,
            surfaces: (
                Self::render_menu_item(
                    font_face,
                    item,
                    desc_width,
                    accel_width,
                    total_width,
                    total_height,
                    false,
                )?,
                Self::render_menu_item(
                    font_face,
                    item,
                    desc_width,
                    accel_width,
                    total_width,
                    total_height,
                    true,
                )?,
            ),
        })
    }

    fn render_menu_item(
        font_face: &FontFace,
        item: &Action,
        desc_width: i32,
        accel_width: i32,
        total_width: i32,
        total_height: i32,
        hovered: bool,
    ) -> Result<ImageSurface, cairo::Error> {
        let surface = ImageSurface::create(Format::ARgb32, total_width, total_height)?;
        let ctx = Context::new(surface.clone())?;
        ctx.set_font_face(font_face);
        ctx.set_font_size(Self::FONT_SIZE);

        if hovered {
            ctx.set_source_rgb(0.2, 0.2, 0.2);
        } else {
            ctx.set_source_rgb(0.1, 0.1, 0.1);
        }
        ctx.paint()?;

        let mut x = Self::PADDING_X;

        {
            ctx.save()?;
            if let Some(color) = item.swatch() {
                ctx.set_source_rgb(
                    color.r as f64 / 255.0,
                    color.g as f64 / 255.0,
                    color.b as f64 / 255.0,
                );
                ctx.translate(
                    x as f64,
                    (total_height as f64 - Self::SWATCH_SIZE as f64) / 2.0,
                );
                ctx.rectangle(0.0, 0.0, Self::SWATCH_SIZE as f64, Self::SWATCH_SIZE as f64);
                ctx.fill()?;
            }
            ctx.restore()?;
            x += Self::SWATCH_SIZE;
        }

        x += Self::PADDING_X;

        {
            ctx.save()?;
            ctx.set_source_rgb(0.9, 0.9, 0.9);
            let extents = ctx.text_extents(item.desc)?;
            ctx.translate(x as f64, (total_height as f64 - extents.height()) / 2.0);
            ctx.move_to(extents.x_bearing(), -extents.y_bearing());
            ctx.show_text(item.desc)?;
            ctx.restore()?;
            x += desc_width;
        }

        x += Self::PADDING_X;

        {
            ctx.save()?;
            ctx.set_source_rgb(0.7, 0.7, 0.7);
            let extents = ctx.text_extents(item.accel_label)?;
            ctx.translate(x as f64, (total_height as f64 - extents.height()) / 2.0);
            // justify the accel label to the right edge of the menu item
            ctx.move_to(
                -extents.x_bearing() + (accel_width as f64 - extents.width()),
                -extents.y_bearing(),
            );
            ctx.show_text(item.accel_label)?;
            ctx.restore()?;
        }

        Ok(surface)
    }

    fn max_width(ctx: &Context, texts: &[&str]) -> Result<i32, cairo::Error> {
        let mut max_width = 0;
        for s in texts {
            let extents = ctx.text_extents(s)?;
            max_width = max_width.max(extents.width().ceil() as i32);
        }
        Ok(max_width)
    }

    fn text_height(ctx: &Context, text: &str) -> Result<i32, cairo::Error> {
        let extents = ctx.text_extents(text)?;
        Ok(extents.height().ceil() as i32)
    }

    /// Draw the context menu at the given position. The menu is drawn as a
    /// vertical list of items, each item is a rectangle with a label and an
    /// optional swatch of color.
    ///
    /// By default the menu is drawn below the cursor, but if there is not
    /// enough space, it is drawn above the cursor.
    pub fn calc_rect(
        items: &[MenuItem],
        pos: Point,
        screen_width: i32,
        screen_height: i32,
    ) -> RectangleInt {
        let total_width = items.iter().map(|item| item.width()).max().unwrap();
        let total_height = items.iter().map(|item| item.height()).sum::<i32>();
        let draw_above = pos.y as i32 + total_height > screen_height;

        // also shift the menu up by 1 pixel to avoid overlapping the cursor
        let mut start_y = if draw_above {
            pos.y as i32 - total_height - 1
        } else {
            pos.y as i32 + 1
        };
        if start_y < 0 {
            start_y = 0;
        }

        let mut start_x = pos.x as i32 - total_width / 2;
        if start_x + total_width > screen_width {
            start_x = screen_width - total_width;
        }
        if start_x < 0 {
            start_x = 0;
        }

        RectangleInt::new(start_x, start_y, total_width, total_height)
    }

    pub fn render(&self, ctx: &Context) -> Result<(), cairo::Error> {
        let mut start_y = self.rect.y() as f64;
        for item in &self.items {
            let surface = item.surface(self.hover == Some(item.idx));
            ctx.set_source_surface(surface, self.rect.x() as f64, start_y)?;
            ctx.rectangle(
                self.rect.x() as f64,
                start_y,
                surface.width() as f64,
                surface.height() as f64,
            );
            ctx.fill()?;
            start_y += surface.height() as f64;
        }
        Ok(())
    }

    /// Update the hovered menu item based on the pointer position. Returns true
    /// if the hovered item changed, false otherwise.
    pub fn update_hover(&mut self, pos: Point) -> bool {
        let mut y = self.rect.y();
        for item in &self.items {
            let rect = RectangleInt::new(self.rect.x(), y, item.width(), item.height());
            if rect_contains(rect, pos.x as i32, pos.y as i32) {
                if self.hover != Some(item.idx) {
                    self.hover = Some(item.idx);
                    return true;
                } else {
                    return false;
                }
            }
            y += item.height();
        }
        if self.hover.is_some() {
            self.hover = None;
            return true;
        }
        false
    }
}

fn rect_contains(rect: RectangleInt, x: i32, y: i32) -> bool {
    x >= rect.x() && x < rect.x() + rect.width() && y >= rect.y() && y < rect.y() + rect.height()
}

pub struct MenuItem {
    idx: usize,
    surfaces: (ImageSurface, ImageSurface), // normal and hovered state surfaces
}

impl MenuItem {
    fn surface(&self, hovered: bool) -> &ImageSurface {
        if hovered {
            &self.surfaces.1
        } else {
            &self.surfaces.0
        }
    }

    fn width(&self) -> i32 {
        self.surfaces.0.width()
    }

    fn height(&self) -> i32 {
        self.surfaces.0.height()
    }
}

pub struct UI {
    surface: ImageSurface,
    context_menu: Option<ContextMenu>,
    last_pointer_pos: Option<Point>,
    actions: &'static [Action],
}

impl UI {
    pub fn new(width: i32, height: i32, actions: &'static [Action]) -> Result<Self, cairo::Error> {
        Ok(Self {
            surface: ImageSurface::create(Format::ARgb32, width, height)?,
            context_menu: None,
            last_pointer_pos: None,
            actions,
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
                // open context menu
                self.context_menu = Some(ContextMenu::new(
                    pos,
                    self.actions,
                    self.surface.width(),
                    self.surface.height(),
                )?);
                self.render()?;
                return Ok((None, true));
            }
            // no menu, and not right-click, so nothing to do
            return Ok((None, false));
        };
        // menu was open, trigger an action if the click was on a menu item
        menu.update_hover(pos);
        self.render()?;
        Ok((menu.hover.map(|idx| self.actions[idx].op), true))
    }

    pub fn on_pointer_button_released(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> Result<(Option<Op>, bool), cairo::Error> {
        let hover = {
            let Some(menu) = self.context_menu.as_mut() else {
                // no menu, so nothing to do
                return Ok((None, false));
            };
            menu.update_hover(pos);
            menu.hover
        };
        let op = if btn == InputButton::Secondary {
            // menu is open and right-click released, trigger action if hovering over an item
            hover.map(|idx| self.actions[idx].op)
        } else {
            // menu is open but not right-click, just close the menu without triggering an action
            self.context_menu = None;
            None
        };
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
        let ctx = Context::new(&self.surface)?;
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
                self.actions,
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
