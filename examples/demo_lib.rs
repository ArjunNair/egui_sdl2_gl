use egui_sdl2_gl::{
    egui,
    mainloop::{run_app, AppInitOptions},
    sdl2,
    sdl2::video::GLProfile,
};

struct DemoLib {
    demo_windows: egui_demo_lib::DemoWindows,
}

impl egui_sdl2_gl::App for DemoLib {
    const INIT_OPTIONS: AppInitOptions = AppInitOptions {
        title: "Demo: Egui backend for SDL2 + GL",
        vsync_enabled: true,
        ..AppInitOptions::DEFAULT
    };

    fn update(&mut self, egui_ctx: &egui::Context, _bctx: egui_sdl2_gl::mainloop::BackendContext) {
        self.demo_windows.ui(egui_ctx);
    }

    fn set_gl_attr(gl_attr: sdl2::video::gl_attr::GLAttr) {
        gl_attr.set_context_profile(GLProfile::Core);
        // Let OpenGL know we are dealing with SRGB colors so that it
        // can do the blending correctly. Not setting the framebuffer
        // leads to darkened, oversaturated colors.
        gl_attr.set_framebuffer_srgb_compatible(true);
        gl_attr.set_double_buffer(true);
        gl_attr.set_multisample_samples(4);

        // OpenGL 3.2 is the minimum that we will support.
        gl_attr.set_context_version(3, 2);

        debug_assert_eq!(gl_attr.context_profile(), GLProfile::Core);
        debug_assert_eq!(gl_attr.context_version(), (3, 2));
    }
}

fn main() {
    run_app(DemoLib {
        demo_windows: Default::default(),
    });
}
