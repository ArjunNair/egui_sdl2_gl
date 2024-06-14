#![warn(clippy::all)]
#![allow(clippy::single_match)]

// Re-export dependencies.
pub use egui;
pub use gl;
pub use sdl2;
pub mod painter;
#[cfg(feature = "use_epi")]
pub use epi;
use painter::Painter;
#[cfg(feature = "use_epi")]
use std::time::Instant;
use {
    egui::*,
    sdl2::{
        event::WindowEvent,
        keyboard::{Keycode, Mod},
        mouse::MouseButton,
        mouse::{Cursor, SystemCursor},
    },
};
#[cfg(feature = "use_epi")]
/// Frame time for CPU usage.
pub fn get_frame_time(start_time: Instant) -> f32 {
    (Instant::now() - start_time).as_secs_f64() as f32
}
#[cfg(feature = "use_epi")]
pub struct Signal;
#[cfg(feature = "use_epi")]
impl Default for Signal {
    fn default() -> Self {
        Self {}
    }
}
#[cfg(feature = "use_epi")]
use epi::backend::RepaintSignal;
#[cfg(feature = "use_epi")]
impl RepaintSignal for Signal {
    fn request_repaint(&self) {}
}

pub struct FusedCursor {
    pub cursor: Cursor,
    pub icon: SystemCursor,
}

impl FusedCursor {
    pub fn new() -> Self {
        Self {
            cursor: Cursor::from_system(SystemCursor::Arrow).unwrap(),
            icon: SystemCursor::Arrow,
        }
    }
}

impl Default for FusedCursor {
    fn default() -> Self {
        Self::new()
    }
}

pub enum DpiScaling {
    /// Default is handled by sdl2, probably 1.0
    Default,
    /// Custome DPI scaling, e.g: 0.8, 1.5, 2.0 and so fort.
    Custom(f32),
}

#[derive(Clone)]
pub enum ShaderVersion {
    /// Default is GLSL 150+.
    Default,
    /// support GLSL 140+ and GLES SL 300.
    Adaptive,
}

pub struct EguiStateHandler {
    pub fused_cursor: FusedCursor,
    pub pointer_pos: Pos2,
    pub input: RawInput,
    pub modifiers: Modifiers,
    pub native_pixels_per_point: f32,
}

pub fn with_sdl2(
    window: &sdl2::video::Window,
    shader_ver: ShaderVersion,
    scale: DpiScaling,
) -> (Painter, EguiStateHandler) {
    let standard_dpi = 96.0;
    let display_dpi =
        window
            .subsystem()
            .display_dpi(0)
            .unwrap_or((standard_dpi, standard_dpi, standard_dpi));
    let dpi_scale = standard_dpi / display_dpi.0;
    let normalized_scale = 1.0 / dpi_scale;
    let default_scale = dpi_scale * normalized_scale;
    let scale = match scale {
        DpiScaling::Default => default_scale,
        DpiScaling::Custom(custom) => default_scale * custom,
    };
    let painter = painter::Painter::new(window, scale, shader_ver);
    let state_handler = EguiStateHandler::new(&painter);
    (painter, state_handler)
}

impl EguiStateHandler {
    pub fn new(painter: &Painter) -> Self {
        let mut input = egui::RawInput {
            screen_rect: Some(painter.screen_rect),
            ..Default::default()
        };
        input
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(painter.pixels_per_point);
        let native_pixels_per_point = painter.pixels_per_point;
        Self {
            fused_cursor: FusedCursor::default(),
            pointer_pos: Pos2::new(0f32, 0f32),
            input,
            modifiers: Modifiers::default(),
            native_pixels_per_point,
        }
    }

    pub fn process_input(
        &mut self,
        window: &sdl2::video::Window,
        event: sdl2::event::Event,
        painter: &mut Painter,
    ) {
        input_to_egui(window, event, painter, self);
    }

    pub fn process_output(
        &mut self,
        window: &sdl2::video::Window,
        egui_output: &egui::PlatformOutput,
    ) {
        if !egui_output.copied_text.is_empty() {
            let copied_text = egui_output.copied_text.clone();
            {
                let result = window
                    .subsystem()
                    .clipboard()
                    .set_clipboard_text(&copied_text);
                if result.is_err() {
                    dbg!("Unable to set clipboard content to SDL clipboard.");
                }
            }
        }
        translate_cursor(&mut self.fused_cursor, egui_output.cursor_icon);
    }
}

pub fn input_to_egui(
    window: &sdl2::video::Window,
    event: sdl2::event::Event,
    painter: &mut Painter,
    state: &mut EguiStateHandler,
) {
    use sdl2::event::Event::*;

    let pixels_per_point = painter.pixels_per_point;
    if event.get_window_id() != Some(window.id()) {
        return;
    }
    match event {
        // handle when window Resized and SizeChanged.
        Window { win_event, .. } => match win_event {
            WindowEvent::Resized(_, _) | sdl2::event::WindowEvent::SizeChanged(_, _) => {
                painter.update_screen_rect(window.drawable_size());
                state.input.screen_rect = Some(painter.screen_rect);
            }
            _ => (),
        },

        //MouseButonLeft pressed is the only one needed by egui
        MouseButtonDown { mouse_btn, .. } => {
            let mouse_btn = match mouse_btn {
                MouseButton::Left => Some(egui::PointerButton::Primary),
                MouseButton::Middle => Some(egui::PointerButton::Middle),
                MouseButton::Right => Some(egui::PointerButton::Secondary),
                _ => None,
            };
            if let Some(pressed) = mouse_btn {
                state.input.events.push(egui::Event::PointerButton {
                    pos: state.pointer_pos,
                    button: pressed,
                    pressed: true,
                    modifiers: state.modifiers,
                });
            }
        }

        //MouseButonLeft pressed is the only one needed by egui
        MouseButtonUp { mouse_btn, .. } => {
            let mouse_btn = match mouse_btn {
                MouseButton::Left => Some(egui::PointerButton::Primary),
                MouseButton::Middle => Some(egui::PointerButton::Middle),
                MouseButton::Right => Some(egui::PointerButton::Secondary),
                _ => None,
            };
            if let Some(released) = mouse_btn {
                state.input.events.push(egui::Event::PointerButton {
                    pos: state.pointer_pos,
                    button: released,
                    pressed: false,
                    modifiers: state.modifiers,
                });
            }
        }

        MouseMotion { x, y, .. } => {
            state.pointer_pos = pos2(x as f32 / pixels_per_point, y as f32 / pixels_per_point);
            state
                .input
                .events
                .push(egui::Event::PointerMoved(state.pointer_pos));
        }

        KeyUp {
            keycode,
            keymod,
            repeat,
            ..
        } => {
            let key_code = match keycode {
                Some(key_code) => key_code,
                _ => return,
            };
            let key = match translate_virtual_key_code(key_code) {
                Some(key) => key,
                _ => return,
            };
            state.modifiers = Modifiers {
                alt: (keymod & Mod::LALTMOD == Mod::LALTMOD)
                    || (keymod & Mod::RALTMOD == Mod::RALTMOD),
                ctrl: (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD)
                    || (keymod & Mod::RCTRLMOD == Mod::RCTRLMOD),
                shift: (keymod & Mod::LSHIFTMOD == Mod::LSHIFTMOD)
                    || (keymod & Mod::RSHIFTMOD == Mod::RSHIFTMOD),
                mac_cmd: keymod & Mod::LGUIMOD == Mod::LGUIMOD,

                //TOD: Test on both windows and mac
                command: (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD)
                    || (keymod & Mod::LGUIMOD == Mod::LGUIMOD),
            };

            state.input.events.push(Event::Key {
                key,
                pressed: false,
                repeat,
                modifiers: state.modifiers,
                // TODO: implement support for physical_key
                physical_key: None,
            });
        }

        KeyDown {
            keycode,
            keymod,
            repeat,
            ..
        } => {
            let key_code = match keycode {
                Some(key_code) => key_code,
                _ => return,
            };

            let key = match translate_virtual_key_code(key_code) {
                Some(key) => key,
                _ => return,
            };
            state.modifiers = Modifiers {
                alt: (keymod & Mod::LALTMOD == Mod::LALTMOD)
                    || (keymod & Mod::RALTMOD == Mod::RALTMOD),
                ctrl: (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD)
                    || (keymod & Mod::RCTRLMOD == Mod::RCTRLMOD),
                shift: (keymod & Mod::LSHIFTMOD == Mod::LSHIFTMOD)
                    || (keymod & Mod::RSHIFTMOD == Mod::RSHIFTMOD),
                mac_cmd: keymod & Mod::LGUIMOD == Mod::LGUIMOD,

                //TOD: Test on both windows and mac
                command: (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD)
                    || (keymod & Mod::LGUIMOD == Mod::LGUIMOD),
            };

            state.input.events.push(Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers: state.modifiers,
                // TODO: implement support for physical_key
                physical_key: None,
            });

            if state.modifiers.command && key == Key::C {
                // println!("copy event");
                state.input.events.push(Event::Copy);
            } else if state.modifiers.command && key == Key::X {
                // println!("cut event");
                state.input.events.push(Event::Cut);
            } else if state.modifiers.command && key == Key::V {
                // println!("paste");
                if let Ok(contents) = window.subsystem().clipboard().clipboard_text() {
                    state.input.events.push(Event::Text(contents));
                }
            }
        }

        TextInput { text, .. } => {
            state.input.events.push(Event::Text(text));
        }

        MouseWheel { x, y, .. } => {
            let delta = vec2(x as f32 * 8.0, y as f32 * 8.0);
            let sdl = window.subsystem().sdl();
            if sdl.keyboard().mod_state() & Mod::LCTRLMOD == Mod::LCTRLMOD
                || sdl.keyboard().mod_state() & Mod::RCTRLMOD == Mod::RCTRLMOD
            {
                state
                    .input
                    .events
                    .push(Event::Zoom((delta.y / 125.0).exp()));
            } else {
                state.input.events.push(Event::Scroll(delta));
            }
        }

        _ => {
            //dbg!(event);
        }
    }
}

pub fn translate_virtual_key_code(key: Keycode) -> Option<egui::Key> {
    Some(match key {
        Keycode::Left => Key::ArrowLeft,
        Keycode::Up => Key::ArrowUp,
        Keycode::Right => Key::ArrowRight,
        Keycode::Down => Key::ArrowDown,

        Keycode::Escape => Key::Escape,
        Keycode::Tab => Key::Tab,
        Keycode::Backspace => Key::Backspace,
        Keycode::Space => Key::Space,
        Keycode::Return => Key::Enter,

        Keycode::Insert => Key::Insert,
        Keycode::Home => Key::Home,
        Keycode::Delete => Key::Delete,
        Keycode::End => Key::End,
        Keycode::PageDown => Key::PageDown,
        Keycode::PageUp => Key::PageUp,

        Keycode::Kp0 | Keycode::Num0 => Key::Num0,
        Keycode::Kp1 | Keycode::Num1 => Key::Num1,
        Keycode::Kp2 | Keycode::Num2 => Key::Num2,
        Keycode::Kp3 | Keycode::Num3 => Key::Num3,
        Keycode::Kp4 | Keycode::Num4 => Key::Num4,
        Keycode::Kp5 | Keycode::Num5 => Key::Num5,
        Keycode::Kp6 | Keycode::Num6 => Key::Num6,
        Keycode::Kp7 | Keycode::Num7 => Key::Num7,
        Keycode::Kp8 | Keycode::Num8 => Key::Num8,
        Keycode::Kp9 | Keycode::Num9 => Key::Num9,

        Keycode::A => Key::A,
        Keycode::B => Key::B,
        Keycode::C => Key::C,
        Keycode::D => Key::D,
        Keycode::E => Key::E,
        Keycode::F => Key::F,
        Keycode::G => Key::G,
        Keycode::H => Key::H,
        Keycode::I => Key::I,
        Keycode::J => Key::J,
        Keycode::K => Key::K,
        Keycode::L => Key::L,
        Keycode::M => Key::M,
        Keycode::N => Key::N,
        Keycode::O => Key::O,
        Keycode::P => Key::P,
        Keycode::Q => Key::Q,
        Keycode::R => Key::R,
        Keycode::S => Key::S,
        Keycode::T => Key::T,
        Keycode::U => Key::U,
        Keycode::V => Key::V,
        Keycode::W => Key::W,
        Keycode::X => Key::X,
        Keycode::Y => Key::Y,
        Keycode::Z => Key::Z,

        _ => {
            return None;
        }
    })
}

pub fn translate_cursor(fused: &mut FusedCursor, cursor_icon: egui::CursorIcon) {
    let tmp_icon = match cursor_icon {
        CursorIcon::Crosshair => SystemCursor::Crosshair,
        CursorIcon::Default => SystemCursor::Arrow,
        CursorIcon::Grab => SystemCursor::Hand,
        CursorIcon::Grabbing => SystemCursor::SizeAll,
        CursorIcon::Move => SystemCursor::SizeAll,
        CursorIcon::PointingHand => SystemCursor::Hand,
        CursorIcon::ResizeHorizontal => SystemCursor::SizeWE,
        CursorIcon::ResizeNeSw => SystemCursor::SizeNESW,
        CursorIcon::ResizeNwSe => SystemCursor::SizeNWSE,
        CursorIcon::ResizeVertical => SystemCursor::SizeNS,
        CursorIcon::Text => SystemCursor::IBeam,
        CursorIcon::NotAllowed | CursorIcon::NoDrop => SystemCursor::No,
        CursorIcon::Wait => SystemCursor::Wait,
        //There doesn't seem to be a suitable SDL equivalent...
        _ => SystemCursor::Arrow,
    };

    if tmp_icon != fused.icon {
        fused.cursor = Cursor::from_system(tmp_icon).unwrap();
        fused.icon = tmp_icon;
        fused.cursor.set();
    }
}
