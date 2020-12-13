#![warn(clippy::all)]
#![allow(clippy::single_match)] 

mod painter;

pub use painter::Painter;

use {
    clipboard::ClipboardProvider,
    egui::*,
    sdl2::{event::WindowEvent, mouse::MouseButton, mouse::SystemCursor, keyboard::{Keycode, Mod} }
};

use clipboard::ClipboardContext; // TODO: remove

pub fn input_to_egui(
    event: sdl2::event::Event,
    clipboard: Option<&mut ClipboardContext>,
    raw_input: &mut RawInput,
) {
    use sdl2::event::Event::*;
    match event {
        //Only the window resize event is handled
        Window {win_event: WindowEvent::Resized(width, height), ..} => {
                    raw_input.screen_size = egui::vec2(width as f32, height as f32)
                        / raw_input.pixels_per_point.unwrap();
        }

        //MouseButonLeft pressed is the only one needed by egui
        MouseButtonDown {mouse_btn: MouseButton::Left, .. } => {
            raw_input.mouse_down = true;
        }

        //MouseButonLeft pressed is the only one needed by egui
        MouseButtonUp {mouse_btn: MouseButton::Left, .. } => {
            raw_input.mouse_down = false;
        }

        MouseMotion {x, y, .. } => {
            raw_input.mouse_pos = Some(pos2(
                x as f32 / raw_input.pixels_per_point.unwrap(),
                y as f32 / raw_input.pixels_per_point.unwrap(),
            ));
        }

        KeyDown {keycode, keymod, .. } => {
            if let Some(key) = translate_virtual_key_code(keycode.unwrap()) {
                raw_input.events.push(Event::Key {
                    key,
                    pressed: true,
                    modifiers: Modifiers {
                       alt: (keymod & Mod::LALTMOD == Mod::LALTMOD) || (keymod & Mod::RALTMOD == Mod::RALTMOD),
                       ctrl:  (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD) || (keymod & Mod::RCTRLMOD == Mod::RCTRLMOD),
                       shift:  (keymod & Mod::LSHIFTMOD == Mod::LSHIFTMOD) || (keymod & Mod::RSHIFTMOD == Mod::RSHIFTMOD),
                       mac_cmd: keymod & Mod::LGUIMOD == Mod::LGUIMOD,

                       //TOD: Test on both windows and mac
                       command:  (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD) || (keymod & Mod::LGUIMOD == Mod::LGUIMOD)
                    }
                });
            }
        }

        KeyUp {keycode, keymod, .. } => {
            match keycode.unwrap() {
                Keycode::Cut => {
                    raw_input.events.push(Event::Cut)
                }
                Keycode::Copy => {
                    raw_input.events.push(Event::Copy)
                }
                Keycode::Paste => {
                    if let Some(clipboard) = clipboard {
                            match clipboard.get_contents() {
                                Ok(contents) => {
                                    raw_input.events.push(Event::Text(contents));
                                }
                                Err(err) => {
                                    eprintln!("Paste error: {}", err);
                                }
                            }
                        }
                }
                _ => {
                    if let Some(key) = translate_virtual_key_code(keycode.unwrap()) {
                        raw_input.events.push(Event::Key {
                            key,
                            pressed: false,
                                modifiers: Modifiers {
                                alt: (keymod & Mod::LALTMOD == Mod::LALTMOD) || (keymod & Mod::RALTMOD == Mod::RALTMOD),
                                ctrl:  (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD) || (keymod & Mod::RCTRLMOD == Mod::RCTRLMOD),
                                shift:  (keymod & Mod::LSHIFTMOD == Mod::LSHIFTMOD) || (keymod & Mod::RSHIFTMOD == Mod::RSHIFTMOD),
                                mac_cmd: keymod & Mod::LGUIMOD == Mod::LGUIMOD,

                                //TOD: Test on both windows and mac
                                command:  (keymod & Mod::LCTRLMOD == Mod::LCTRLMOD) || (keymod & Mod::LGUIMOD == Mod::LGUIMOD)
                            }
                        });
                    }
                }
            }
        }

        MouseWheel { x, y, .. } => {
            raw_input.scroll_delta = vec2(x as f32, y as f32);
        }

        _ => {
            // dbg!(event);
        }
    }
}

pub fn translate_virtual_key_code(key: sdl2::keyboard::Keycode) -> Option<egui::Key> {
    use Keycode::*;

    Some(match key {
        Escape => Key::Escape,
        Insert => Key::Insert,
        Home => Key::Home,
        Delete => Key::Delete,
        End => Key::End,
        PageDown => Key::PageDown,
        PageUp => Key::PageUp,
        Left => Key::ArrowLeft,
        Up => Key::ArrowUp,
        Right => Key::ArrowRight,
        Down => Key::ArrowDown,
        Backspace => Key::Backspace,
        Return => Key::Enter,
        // Space => Key::Space,
        Tab => Key::Tab,
        _ => {
            return None;
        }
    })
}

pub fn translate_cursor(cursor_icon: egui::CursorIcon) -> sdl2::mouse::SystemCursor{
    match cursor_icon {
        CursorIcon::Default => SystemCursor::Arrow,
        CursorIcon::PointingHand => SystemCursor::Hand,
        CursorIcon::ResizeHorizontal => SystemCursor::SizeWE,
        CursorIcon::ResizeNeSw => SystemCursor::SizeNESW,
        CursorIcon::ResizeNwSe => SystemCursor::SizeNWSE,
        CursorIcon::ResizeVertical => SystemCursor::SizeNS,
        CursorIcon::Text => SystemCursor::IBeam,

        //There doesn't seem to be a suitable SDL equivalent...
        CursorIcon::Grab => SystemCursor::Hand,
        CursorIcon::Grabbing => SystemCursor::Hand
    }
}

pub fn init_clipboard() -> Option<ClipboardContext> {
    match ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            eprintln!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}
