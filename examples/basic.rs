use egui::Checkbox;
use egui_backend::DpiScaling;
use std::time::Instant;
// Alias the backend to something less mouthful
use egui_sdl2_gl as egui_backend;
use sdl2::{
    event::Event,
    video::{GLProfile, SwapInterval},
};

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::Core);
    // On linux, OpenGL ES Mesa driver 22.0.0+ can be used like so:
    // gl_attr.set_context_profile(GLProfile::GLES);

    gl_attr.set_double_buffer(true);
    gl_attr.set_multisample_samples(4);

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
    gl::load_with(|name| {
        // println!("Loading: {}", name);
        window.subsystem().gl_get_proc_address(name) as *const _
    });

    // Init egui stuff
    let (mut painter, mut egui_state) = egui_backend::with_sdl2(&window, DpiScaling::Custom(2.0));
    let egui_ctx = egui::Context::default();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut test_str: String =
        "A text box to write in. Cut, copy, paste commands are available.".to_owned();

    let mut enable_vsync = false;
    let mut quit = false;
    let mut slider = 0.0;

    let start_time = Instant::now();

    'running: loop {
        if enable_vsync {
            window
                .subsystem()
                .gl_set_swap_interval(SwapInterval::VSync)
                .unwrap()
        } else {
            window
                .subsystem()
                .gl_set_swap_interval(SwapInterval::Immediate)
                .unwrap()
        }

        egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
        egui_ctx.begin_frame(egui_state.input.take());

        egui::CentralPanel::default().show(&egui_ctx, |ui| {
            ui.label(" ");
            ui.text_edit_multiline(&mut test_str);
            ui.label(" ");
            ui.add(egui::Slider::new(&mut slider, 0.0..=50.0).text("Slider"));
            ui.label(" ");
            ui.add(Checkbox::new(&mut enable_vsync, "Reduce CPU Usage?"));
            ui.separator();
            if ui.button("Quit?").clicked() {
                quit = true;
            }
        });

        let full_output = egui_ctx.end_frame();

        // Process ouput
        egui_state.process_output(&window, &full_output.platform_output);

        // For default dpi scaling only, Update window when the size of resized window is very small (to avoid egui::CentralPanel distortions).
        // if egui_ctx.used_size() != painter.screen_rect.size() {
        //     println!("resized.");
        //     let _size = egui_ctx.used_size();
        //     let (w, h) = (_size.x as u32, _size.y as u32);
        //     window.set_size(w, h).unwrap();
        // }

        let paint_jobs = egui_ctx.tessellate(full_output.shapes);

        // An example of how OpenGL can be used to draw custom stuff with egui
        // overlaying it:
        // First clear the background to something nice.
        unsafe {
            // Clear the screen to green
            gl::ClearColor(0.3, 0.6, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        //if !full_output..needs_repaint {
        if !full_output.repaint_after.is_zero() {
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
            //painter.paint_jobs(None, paint_jobs, &egui_ctx.font_image());
            painter.paint_and_update_textures(paint_jobs.as_slice(), &full_output.textures_delta);
            window.gl_swap_window();
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
