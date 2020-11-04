extern crate sdl2;
extern crate gl;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::video::GLProfile;
use std::time::Instant;

use egui::{vec2, color, Srgba, Image};
//use egui_sdl2::Painter;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;
const PIC_WIDTH: i32 = 500;
const PIC_HEIGHT: i32 = 256;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    //let timer_subsystem = sdl_context.timer().unwrap();
    
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    gl_attr.set_context_version(4, 5);

    let window = video_subsystem.window("Demo: Egui backend for SDL2 + GL", SCREEN_WIDTH, SCREEN_HEIGHT)
        .opengl()
        .build()
        .unwrap();

    // Create a window context
    let _ctx = window.gl_create_context().unwrap();

    let mut painter = egui_sdl2::Painter::new(&video_subsystem, SCREEN_WIDTH, SCREEN_HEIGHT);
    let mut egui_ctx = egui::Context::new();

    debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
    debug_assert_eq!(gl_attr.context_version(), (4, 5));

    let mut event_pump = sdl_context.event_pump().unwrap();
    let pixels_per_point = 96f32 / video_subsystem.display_dpi(0).unwrap().0;

    let mut raw_input = egui::RawInput {
        screen_size: {
            let (width, height) = window.size();
            vec2(width as f32, height as f32) / pixels_per_point
        },
        pixels_per_point: Some(pixels_per_point),
        ..Default::default()
    };

    let start_time = Instant::now();

    let mut clipboard = egui_sdl2::init_clipboard();
    let mut srgba: Vec<Srgba> = Vec::new();

     //For now we will just set everything to black, because
     //we will be updating it dynamically later. However, this could just as 
     //easily have been some actual picture data loaded in.
    for _ in 0..PIC_HEIGHT {
        for _ in 0..PIC_WIDTH{
            srgba.push(color::BLACK);
        }
    }

    //The user texture is what allows us to mix Egui and GL rendering contexts.
    //Egui just needs the texture id, as the actual texture is managed by the backend.
    let chip8_tex_id = painter.new_user_texture((PIC_WIDTH as usize, PIC_HEIGHT as usize), &srgba, false);

    //Some variables to help draw a sine wave
    let mut sine_shift = 0f32;
    let mut angle = 0f32;
    let mut amplitude: f32 = 50f32;

    'running: loop {
        raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
        let ui = egui_ctx.begin_frame(raw_input.take());
    
        let mut srgba: Vec<Srgba> = Vec::new();

        //Draw a cool sine wave in a buffer.
        for y in 0..PIC_HEIGHT {
            for x in 0..PIC_WIDTH{
                srgba.push(color::BLACK);
                if y == PIC_HEIGHT - 1 {
                    let y = amplitude * (angle * 3.142f32/180f32 + sine_shift).sin();
                    let y = PIC_HEIGHT as f32/2f32 - y;
                    srgba[ (y as i32 * PIC_WIDTH + x) as usize] = color::YELLOW;
                    angle += 360f32 / PIC_WIDTH as f32;
                }
            }
        }
        sine_shift += 0.1f32;

        //This updates the previously initialized texture with new data.
        //If we weren't updating the texture, this call wouldn't be required.
        painter.update_user_texture_data(chip8_tex_id, &srgba);

        egui::Window::new("Egui with SDL2 events and GL texture").show(ui.ctx(), |ui| {
            //Image just needs a texture id reference, so we just pass it the texture id that was returned to us
            //when we previously initialized the texture.
            ui.add(Image::new(chip8_tex_id, vec2(PIC_WIDTH as f32, PIC_HEIGHT as f32)));
            ui.separator();
            ui.label("A simple sine wave plotted via some probably dodgy math. The GL texture is dynamically updated and blitted to an Egui managed Image.");
            ui.label(" ");
            ui.add(egui::Slider::f32(&mut amplitude, 0.0..=100.0).text("Amplitude"));
            ui.label(" ");
            if ui.button("Quit").clicked {
                std::process::exit(0);
            }
        });
        
        //We aren't handling the output at the moment.
        let (_output, paint_jobs) = egui_ctx.end_frame();

        painter.paint_jobs(color::srgba(0, 0, 255, 128), paint_jobs, &egui_ctx.texture(), pixels_per_point);
        window.gl_swap_window();

        //Using regular SDL2 event pipeline
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {
                    egui_sdl2::input_to_egui(event, clipboard.as_mut(), &mut raw_input);
                }
            }
        }
        std::thread::sleep(::std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
    painter.cleanup();
}