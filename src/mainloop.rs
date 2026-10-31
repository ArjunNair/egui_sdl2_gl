use crate::sdl2::video::{gl_attr::GLAttr, GLProfile, SwapInterval, Window};
use crate::{egui, gl, painter::Painter, sdl2, DpiScaling, ShaderVersion};

pub struct AppInitOptions {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub vsync_enabled: bool,
    /// For default dpi scaling only, Update window when the size of resized window is very small (to avoid egui::CentralPanel distortions).
    pub handle_dpi_scaling: bool,
}

impl AppInitOptions {
    pub const DEFAULT: Self = Self {
        title: "Unnamed Egui + SDL2 + GL app",
        width: 800,
        height: 600,
        vsync_enabled: false,
        handle_dpi_scaling: false,
    };
}

#[allow(dead_code)]
pub struct BackendContext<'a> {
    pub painter: &'a mut Painter,
    pub window: &'a mut Window,
}

fn apply_vsync_state(subsys: &sdl2::VideoSubsystem, enabled: bool) {
    if enabled {
        if let Err(error) = subsys.gl_set_swap_interval(SwapInterval::VSync) {
            log::warn!("Failed to gl_set_swap_interval(SwapInterval::VSync): {error}",);
        }
    } else if let Err(error) = subsys.gl_set_swap_interval(SwapInterval::Immediate) {
        log::warn!("Failed to gl_set_swap_interval(SwapInterval::Immediate): {error}",);
    }
}

impl<'a> BackendContext<'a> {
    pub fn change_vsync_state(&self, enable: bool) {
        log::info!("Changing vsync to enabled={enable}");
        apply_vsync_state(self.window.subsystem(), enable);
    }
}

pub trait App {
    const INIT_OPTIONS: AppInitOptions;

    fn set_gl_attr(gl_attr: GLAttr) {
        // On linux, OpenGL ES Mesa driver 22.0.0+ can be used by setting GLProfile::GLES
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_double_buffer(true);
        gl_attr.set_multisample_samples(4);
    }

    fn gl_frame_init(&self) {
        unsafe {
            gl::ClearColor(0.3, 0.6, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    fn update(&mut self, egui_ctx: &egui::Context, bctx: BackendContext);
}

// NOTE: example/demo_lib made previously use of
// ```
// let frame_time = get_frame_time(start_time);
// let frame = epi::Frame::new(epi::backend::FrameData {
//     info: epi::IntegrationInfo {
//         web_info: None,
//         cpu_usage: Some(frame_time),
//         native_pixels_per_point: Some(egui_state.native_pixels_per_point),
//         prefer_dark_mode: None,
//         name: "egui + sdl2 + gl",
//     },
//     output: Default::default(),
//     repaint_signal: repaint_signal.clone(),
// });
// ```
// just after `egui_ctx.begin_pass` and before `app.update` (`demo.ui`)
// seemingly just to later take `frame.take_app_output().quit`
// repaint_signal was not used anywhere?

pub fn run_app<A: App>(mut app: A) {
    let aio = A::INIT_OPTIONS;

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    A::set_gl_attr(video_subsystem.gl_attr());

    let mut window = video_subsystem
        .window(aio.title, aio.width, aio.height)
        .opengl()
        .resizable()
        .build()
        .unwrap();
    let _window_ctx = window.gl_create_context().unwrap();
    apply_vsync_state(window.subsystem(), aio.vsync_enabled); // apply initial vsync state

    // On linux use GLES SL 100+, via ShaderVersion::Adaptive;
    let (mut painter, mut egui_state) =
        crate::with_sdl2(&window, ShaderVersion::Default, DpiScaling::Default);
    let egui_ctx = egui::Context::default();

    let start_time = std::time::Instant::now();
    let mut should_quit = false;

    while !should_quit {
        egui_state.input.time = Some(start_time.elapsed().as_secs_f64());
        egui_ctx.begin_pass(egui_state.input.take());

        app.gl_frame_init();
        let bctx = BackendContext {
            painter: &mut painter,
            window: &mut window,
        };
        app.update(&egui_ctx, bctx);

        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        } = egui_ctx.end_pass();

        // Process ouput
        egui_state.process_output(&window, &platform_output);

        if aio.handle_dpi_scaling && egui_ctx.used_size() != painter.screen_rect.size() {
            log::info!("resized.");
            let _size = egui_ctx.used_size();
            window.set_size(_size.x as u32, _size.y as u32).unwrap();
        };

        let paint_jobs = egui_ctx.tessellate(shapes, pixels_per_point);
        painter.paint_jobs(None, textures_delta, paint_jobs);
        window.gl_swap_window();

        let root_viewport = viewport_output
            .get(&egui::ViewportId::ROOT)
            .expect("Missing ViewportId::ROOT");
        let mut rv_cmds = root_viewport.commands.iter();
        should_quit = rv_cmds.any(|cmd| cmd.eq(&egui::ViewportCommand::Close));

        if !root_viewport.repaint_delay.is_zero() {
            if let Some(event) = event_pump.wait_event_timeout(5) {
                egui_state.process_input(&window, event, &mut painter, &mut should_quit);
            }
        } else {
            for event in event_pump.poll_iter() {
                egui_state.process_input(&window, event, &mut painter, &mut should_quit);
            }
        }
    }
}
