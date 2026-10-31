use egui::load::SizedTexture;
use egui_backend::egui::{self, vec2, Color32, Image};
use egui_sdl2_gl as egui_backend;
use egui_sdl2_gl::mainloop::{run_app, AppInitOptions};
use sdl2::video::GLProfile;
use std::f32;

mod triangle;

const PIC_WIDTH: usize = 320;
const PIC_HEIGHT: usize = 192;

struct Mix {
    sine_shift: f32,
    amplitude: f32,
    chip8_tex_id: Option<egui::TextureId>,
    triangle: Option<triangle::Triangle>,
}

impl Default for Mix {
    fn default() -> Self {
        Self {
            sine_shift: 0.0,
            amplitude: 50.0,
            chip8_tex_id: None,
            triangle: None,
        }
    }
}

impl egui_sdl2_gl::App for Mix {
    const INIT_OPTIONS: AppInitOptions = AppInitOptions {
        title: "",
        vsync_enabled: true,
        ..AppInitOptions::DEFAULT
    };

    fn update(&mut self, egui_ctx: &egui::Context, bctx: egui_sdl2_gl::mainloop::BackendContext) {
        let Self {
            chip8_tex_id,
            sine_shift,
            amplitude,
            triangle,
        } = self;

        let triangle = triangle.get_or_insert_with(triangle::Triangle::new);
        triangle.draw();

        let mut srgba: Vec<Color32> = Vec::new();
        let mut angle = 0f32;
        // Draw a cool sine wave in a buffer.
        for y in 0..PIC_HEIGHT {
            for x in 0..PIC_WIDTH {
                srgba.push(Color32::BLACK);
                if y == PIC_HEIGHT - 1 {
                    let y = *amplitude * (angle * f32::consts::PI / 180f32 + *sine_shift).sin();
                    let y = PIC_HEIGHT as f32 / 2f32 - y;
                    srgba[y as usize * PIC_WIDTH + x] = Color32::YELLOW;
                    angle += 360f32 / PIC_WIDTH as f32;
                }
            }
        }

        self.sine_shift += 0.1f32;

        // The user texture is what allows us to mix Egui and GL rendering contexts.
        // Egui just needs the texture id, as the actual texture is managed by the backend.
        let _chip8_tex_id = *chip8_tex_id.get_or_insert_with(|| {
            bctx.painter
                .new_user_texture((PIC_WIDTH, PIC_HEIGHT), &srgba, false)
        });

        // This updates the previously initialized texture with new data.
        // If we weren't updating the texture, this call wouldn't be required.
        bctx.painter.update_user_texture_data(_chip8_tex_id, &srgba);

        egui::Window::new("Egui with SDL2 and GL").show(egui_ctx, |ui| {
            // Image just needs a texture id reference, so we just pass it the texture id that was returned to us
            // when we previously initialized the texture.
            ui.add(Image::new(SizedTexture::new(_chip8_tex_id, vec2(PIC_WIDTH as f32, PIC_HEIGHT as f32))));
            ui.separator();
            ui.label("A simple sine wave plotted onto a GL texture then blitted to an egui managed Image.");
            ui.label(" ");
            ui.add(egui::Slider::new(&mut self.amplitude, 0.0..=50.0).text("Amplitude"));
            ui.label(" ");
            if ui.button("Quit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn set_gl_attr(gl_attr: sdl2::video::gl_attr::GLAttr) {
        gl_attr.set_context_profile(GLProfile::Core);

        // Let OpenGL know we are dealing with SRGB colors so that it
        // can do the blending correctly. Not setting the framebuffer
        // leads to darkened, oversaturated colors.
        gl_attr.set_double_buffer(true);
        gl_attr.set_multisample_samples(4);
        gl_attr.set_framebuffer_srgb_compatible(true);

        // OpenGL 3.2 is the minimum that we will support.
        gl_attr.set_context_version(3, 2);
    }
}

fn main() {
    run_app(Mix::default());
}
