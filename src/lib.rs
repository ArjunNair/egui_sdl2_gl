#![warn(clippy::all)]
#![allow(clippy::single_match)] 

mod painter;

pub use painter::Painter;

use {
    clipboard::ClipboardProvider,
    egui::*,
    sdl2::{event::WindowEvent, mouse::MouseButton, mouse::SystemCursor, keyboard::Keycode}
};

pub use clipboard::ClipboardContext; // TODO: remove

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

        KeyDown {keycode, .. } => {
            if let Some(key) = translate_virtual_key_code(keycode.unwrap()) {
                raw_input.events.push(Event::Key {
                    key,
                    pressed: true
                });
            }
        }

        KeyUp {keycode, .. } => {
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
                            pressed: false
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
        Left => Key::Left,
        Up => Key::Up,
        Right => Key::Right,
        Down => Key::Down,
        Backspace => Key::Backspace,
        Return => Key::Enter,
        // Space => Key::Space,
        Tab => Key::Tab,

        LAlt | RAlt => Key::Alt,
        LShift | RShift => Key::Shift,
        LCtrl | RCtrl => Key::Control,
        LGui  => Key::Logo,

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
