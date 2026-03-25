use smithay_client_toolkit::{
    seat::keyboard::Keysym,
    shell::{WaylandSurface, xdg::window::WindowDecorations},
};
use wayland_client::QueueHandle;

use super::{OverlayState, View};
use crate::model::{Color, Command, Tool};

impl View {
    pub(super) fn dispatch_command(&mut self, qh: &QueueHandle<Self>, cmd: Command) {
        let damage = match cmd {
            Command::ShowOverlay => {
                self.create_overlay(qh);
                None
            }
            Command::HideOverlay => {
                self.destroy_overlay();
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
            } => self.draw_line(style, radius, from, to),
            Command::DrawDot {
                style,
                radius,
                center,
            } => self.draw_dot(style, radius, center),
            Command::ClearBuffer => self.clear_buffer(),
            Command::ToggleHelp(_) => {
                let overlay = match self.overlay.as_ref() {
                    Some(OverlayState::Ready(o)) => o,
                    _ => return,
                };
                Some(super::render::DirtyRect::full(
                    overlay.width,
                    overlay.height,
                ))
            }
        };

        if let Some(damage) = damage {
            self.mark_dirty(qh, damage);
        }
    }

    fn create_overlay(&mut self, qh: &QueueHandle<Self>) {
        debug_assert!(
            self.overlay.is_none(),
            "create_overlay called while overlay already exists"
        );
        let surface = self.wayland.compositor_state.create_surface(qh);
        let window = self
            .wayland
            .xdg_shell
            .create_window(surface, WindowDecorations::None, qh);
        window.set_title("Waydoodle");
        window.set_app_id("io.github.dessaya.waydoodle");
        window.set_maximized();
        window.commit();
        self.overlay = Some(OverlayState::Pending(window));
    }

    fn destroy_overlay(&mut self) {
        if let Some(state) = self.overlay.take() {
            let surface = state.window().wl_surface().clone();
            drop(state);
            surface.destroy();
        }
    }

    pub(super) fn handle_key(&mut self, qh: &QueueHandle<Self>, keysym: Keysym) {
        let overlay = match self.model.overlay.as_mut() {
            Some(o) => o,
            None => return,
        };

        let cmd = match keysym {
            Keysym::F1 => Some(overlay.toggle_help()),
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
