use smithay_client_toolkit::{
    seat::keyboard::Keysym,
    shell::{WaylandSurface, xdg::window::WindowDecorations},
};
use wayland_client::QueueHandle;

use super::View;
use crate::model::{Color, Command, Tool};

impl View {
    pub(super) fn dispatch_command(&mut self, qh: &QueueHandle<Self>, cmd: Command) {
        let damage = match cmd {
            Command::ShowOverlay => {
                self.show_overlay(qh);
                None
            }
            Command::HideOverlay => {
                self.hide_overlay();
                None
            }
            Command::SetCursorShape(shape) => {
                self.apply_cursor(shape, qh);
                None
            }
            Command::DrawLine {
                style,
                radius,
                from,
                to,
            } => self.render_line(style, radius, from, to),
            Command::DrawDot {
                style,
                radius,
                center,
            } => self.render_dot(style, radius, center),
            Command::ClearBuffer => self.clear_buffer(),
        };

        if let Some(damage) = damage {
            self.draw_frame(qh, damage);
        }
    }

    fn show_overlay(&mut self, qh: &QueueHandle<Self>) {
        if self.window.is_some() {
            self.hide_overlay();
        }

        let surface = self.wayland.compositor_state.create_surface(qh);
        let window = self
            .wayland
            .xdg_shell
            .create_window(surface, WindowDecorations::None, qh);

        window.set_title("Waydoodle");
        window.set_app_id("io.github.dessaya.waydoodle");
        window.set_maximized();
        window.commit();

        self.window = Some(window);
        self.first_configure = true;
        self.buffer = None;
    }

    pub(super) fn hide_overlay(&mut self) {
        if let Some(window) = self.window.take() {
            let surface = window.wl_surface().clone();
            drop(window);
            surface.destroy();
        }
        self.buffer = None;
        self.pool = None;
        self.width = 0;
        self.height = 0;
    }

    pub(super) fn handle_key(
        &mut self,
        qh: &QueueHandle<Self>,
        keysym: smithay_client_toolkit::seat::keyboard::Keysym,
    ) {
        let overlay = match self.model.overlay.as_mut() {
            Some(o) => o,
            None => return,
        };

        let cmd = match keysym {
            Keysym::r | Keysym::R => Some(overlay.set_tool(Tool::Pen(Color::Red))),
            Keysym::g | Keysym::G => Some(overlay.set_tool(Tool::Pen(Color::Green))),
            Keysym::b | Keysym::B => Some(overlay.set_tool(Tool::Pen(Color::Blue))),
            Keysym::y | Keysym::Y => Some(overlay.set_tool(Tool::Pen(Color::Yellow))),
            Keysym::m | Keysym::M => Some(overlay.set_tool(Tool::Pen(Color::Magenta))),
            Keysym::n | Keysym::N => Some(overlay.set_tool(Tool::Pen(Color::Cyan))),
            Keysym::e | Keysym::E => Some(overlay.set_tool(Tool::Eraser)),
            Keysym::c | Keysym::C => Some(overlay.clear()),
            Keysym::Escape => Some(self.model.hide_overlay()),
            _ => None,
        };

        if let Some(cmd) = cmd {
            self.dispatch_command(qh, cmd);
        }
    }
}
