use egui_sdl2_gl::mainloop::{run_app, AppInitOptions, BackendContext};

#[derive(Default)]
struct TestApp {
    input: String,
    value: f32,
    vsync_enabled: bool,
}

impl egui_sdl2_gl::App for TestApp {
    const INIT_OPTIONS: AppInitOptions = AppInitOptions {
        title: "Demo: Egui backend for SDL2 + GL",
        ..AppInitOptions::DEFAULT
    };

    fn update(&mut self, egui_ctx: &egui::Context, bctx: BackendContext) {
        egui::CentralPanel::default().show(egui_ctx, |ui| {
            ui.label(" ");
            ui.text_edit_multiline(&mut self.input);
            ui.label(" ");
            ui.add(egui::Slider::new(&mut self.value, 0.0..=50.0).text("Slider"));
            ui.label(" ");

            let checkbox = egui::Checkbox::new(&mut self.vsync_enabled, "Reduce CPU Usage?");
            if ui.add(checkbox).changed() {
                bctx.change_vsync_state(self.vsync_enabled);
            }

            ui.separator();
            if ui.button("Quit?").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    run_app(TestApp {
        input: "A text box to write in. Cut, copy, paste commands are available.".to_string(),
        ..Default::default()
    });
}
