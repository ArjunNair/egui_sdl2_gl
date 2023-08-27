use std::time::Instant;
//Alias the backend to something less mouthful
use egui_backend::egui::{vec2, Color32, FullOutput, Image};
use egui_backend::sdl2::video::GLProfile;
use egui_backend::{egui, gl, sdl2};
use egui_backend::{sdl2::event::Event, DpiScaling, ShaderVersion};
use egui_sdl2_gl as egui_backend;
use sdl2::video::SwapInterval;
mod triangle;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;
const PIC_WIDTH: i32 = 320;
const PIC_HEIGHT: i32 = 192;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);

    // Let OpenGL know we are dealing with SRGB colors so that it
    // can do the blending correctly. Not setting the framebuffer
    // leads to darkened, oversaturated colors.
    gl_attr.set_double_buffer(true);
    gl_attr.set_multisample_samples(4);
    gl_attr.set_framebuffer_srgb_compatible(true);

    // OpenGL 3.2 is the minimum that we will support.
    gl_attr.set_context_version(3, 2);

    let window = video_subsystem
        .window(
            "Demo: Egui backend for SDL2 + GL",
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
        )
        .opengl()
        .resizable()
        .build()
        .unwrap();

    // Create a window context
    let _ctx = window.gl_create_context().unwrap();
    debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
    debug_assert_eq!(gl_attr.context_version(), (3, 2));

    // Enable vsync
    window
        .subsystem()
        .gl_set_swap_interval(SwapInterval::VSync)
        .unwrap();

    // Init egui stuff
    let (mut painter, mut egui_state) =
        egui_backend::with_sdl2(&window, ShaderVersion::Default, DpiScaling::Default);
    let mut egui_ctx = egui::Context::default();
    let mut event_pump: sdl2::EventPump = sdl_context.event_pump().unwrap();
    let mut srgba: Vec<Color32> = Vec::new();

    // For now we will just set everything to black, because
    // we will be updating it dynamically later. However, this could just as
    // easily have been some actual picture data loaded in.
    for _ in 0..PIC_HEIGHT {
        for _ in 0..PIC_WIDTH {
            srgba.push(Color32::BLACK);
        }
    }

    // The user texture is what allows us to mix Egui and GL rendering contexts.
    // Egui just needs the texture id, as the actual texture is managed by the backend.
    let chip8_tex_id =
        painter.new_user_texture((PIC_WIDTH as usize, PIC_HEIGHT as usize), &srgba, false);

    // Some variables to help draw a sine wave
    let mut sine_shift = 0f32;
    let mut amplitude: f32 = 50f32;
    let mut test_str: String =
        "A text box to write in. Cut, copy, paste commands are available.".to_owned();
    let start_time = Instant::now();
    // We will draw a crisp white triangle using OpenGL.
    let triangle = triangle::Triangle::new();
    let mut quit = false;

    'running: loop {
        egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
        egui_ctx.begin_frame(egui_state.input.take());

        // An example of how OpenGL can be used to draw custom stuff with egui
        // overlaying it:
        // First clear the background to something nice.
        unsafe {
            // Clear the screen to green
            gl::ClearColor(0.3, 0.6, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        // Then draw our triangle.
        triangle.draw();

        let mut srgba: Vec<Color32> = Vec::new();
        let mut angle = 0f32;
        // Draw a cool sine wave in a buffer.
        for y in 0..PIC_HEIGHT {
            for x in 0..PIC_WIDTH {
                srgba.push(Color32::BLACK);
                if y == PIC_HEIGHT - 1 {
                    let y = amplitude * (angle * 3.142f32 / 180f32 + sine_shift).sin();
                    let y = PIC_HEIGHT as f32 / 2f32 - y;
                    srgba[(y as i32 * PIC_WIDTH + x) as usize] = Color32::YELLOW;
                    angle += 360f32 / PIC_WIDTH as f32;
                }
            }
        }

        sine_shift += 0.1f32;

        // This updates the previously initialized texture with new data.
        // If we weren't updating the texture, this call wouldn't be required.
        painter.update_user_texture_data(chip8_tex_id, &srgba);

        egui::Window::new("Egui with SDL2 and GL").show(&egui_ctx, |ui| {
            // Image just needs a texture id reference, so we just pass it the texture id that was returned to us
            // when we previously initialized the texture.
            ui.add(Image::new(chip8_tex_id, vec2(PIC_WIDTH as f32, PIC_HEIGHT as f32)));
            ui.separator();
            ui.label("A simple sine wave plotted onto a GL texture then blitted to an egui managed Image.");
            ui.label(" ");
            ui.text_edit_multiline(&mut test_str);
            ui.label(" ");
            ui.add(egui::Slider::new(&mut amplitude, 0.0..=50.0).text("Amplitude"));
            ui.label(" ");
            if ui.button("Quit").clicked() {
                quit = true;
            }
        });

        let FullOutput{platform_output, repaint_after, textures_delta, shapes} = egui_ctx.end_frame();
        // Process ouput
        egui_state.process_output(&window, &platform_output);

        let paint_jobs = egui_ctx.tessellate(shapes);

        // Note: passing a bg_color to paint_jobs will clear any previously drawn stuff.
        // Use this only if egui is being used for all drawing and you aren't mixing your own Open GL
        // drawing calls with it.
        // Since we are custom drawing an OpenGL Triangle we don't need egui to clear the background.
        painter.paint_jobs(None, textures_delta, paint_jobs);

        window.gl_swap_window();

        if repaint_after.is_zero() {
            if let Some(event) = event_pump.wait_event_timeout(5) {
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => {
                        // Process input event
                        egui_state.process_input(&window, event, &mut painter);
                    }
                }
            }
        } else {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => {
                        // Process input event
                        egui_state.process_input(&window, event, &mut painter);
                    }
                }
            }
        }

        if quit {
            break;
        }
    }
}
