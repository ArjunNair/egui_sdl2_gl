extern crate gl;
extern crate sdl2;
use crate::ShaderVersion;
use ahash::AHashMap;
use core::mem;
use core::ptr;
use core::str;
use egui::{
    epaint::{Color32, Mesh, Primitive},
    vec2, ClippedPrimitive, Pos2, Rect,
};
use gl::types::{GLchar, GLenum, GLint, GLsizeiptr, GLsync, GLuint};
use std::convert::TryInto;
use std::ffi::CString;

const DEFAULT_VERT_SRC: &str = include_str!("../shaders/default.vert");
const DEFAULT_FRAG_SRC: &str = include_str!("../shaders/default.frag");
const ADAPTIVE_VERT_SRC: &str = include_str!("../shaders/adaptive.vert");
const ADAPTIVE_FRAG_SRC: &str = include_str!("../shaders/adaptive.frag");

#[derive(Default)]
struct Texture {
    size: (usize, usize),

    /// Pending upload (will be emptied later).
    pixels: Vec<u8>,

    /// Lazily uploaded
    gl_id: Option<GLuint>,

    /// For user textures there is a choice between
    /// Linear (default) and Nearest.
    filtering: bool,

    /// User textures can be modified and this flag
    /// is used to indicate if pixel data for the
    /// texture has been updated.
    dirty: bool,
}

pub struct Painter {
    vertex_array: GLuint,
    program: GLuint,
    index_buffer: GLuint,
    vertex_buffer: GLuint,
    // Call fence for sdl vsync so the CPU won't heat up if there's no heavy activity.
    pub gl_sync_fence: GLsync,
    textures: AHashMap<egui::TextureId, Texture>,
    pub pixels_per_point: f32,
    pub canvas_size: (u32, u32),
    pub screen_rect: Rect,
}

macro_rules! get_gl_error {
    ($id:expr, $fnlen:ident, $fnlog:ident) => {{
        let mut len = 0;
        unsafe {
            gl::$fnlen($id, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            gl::$fnlog($id, len, ptr::null_mut(), buf.as_mut_ptr() as *mut GLchar);
            buf.set_len(len.try_into().unwrap());
            CString::from_vec_with_nul(buf)
                .unwrap()
                .to_string_lossy()
                .to_string()
        }
    }};
}

fn get_shader_error(id: u32) -> String {
    get_gl_error!(id, GetShaderiv, GetShaderInfoLog)
}

fn get_program_error(id: u32) -> String {
    get_gl_error!(id, GetProgramiv, GetProgramInfoLog)
}

pub fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);
        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            panic!("{}", get_shader_error(shader));
        }
    }
    shader
}

pub fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            panic!("{}", get_program_error(program));
        }
        program
    }
}

impl Painter {
    pub fn new(window: &sdl2::video::Window, scale: f32, shader_ver: ShaderVersion) -> Painter {
        unsafe {
            gl::load_with(|name| window.subsystem().gl_get_proc_address(name) as *const _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            let (vs_src, fs_src) = match shader_ver {
                ShaderVersion::Default => (DEFAULT_VERT_SRC, DEFAULT_FRAG_SRC),
                ShaderVersion::Adaptive => (ADAPTIVE_VERT_SRC, ADAPTIVE_FRAG_SRC),
            };
            let vs_src = CString::new(vs_src).unwrap().to_string_lossy().to_string();
            let fs_src = CString::new(fs_src).unwrap().to_string_lossy().to_string();
            let vert_shader = compile_shader(&vs_src, gl::VERTEX_SHADER);
            let frag_shader = compile_shader(&fs_src, gl::FRAGMENT_SHADER);

            let program = link_program(vert_shader, frag_shader);
            let mut vertex_array = 0;
            let mut index_buffer = 0;
            let mut vertex_buffer = 0;
            gl::GenVertexArrays(1, &mut vertex_array);
            gl::BindVertexArray(vertex_array);
            assert!(vertex_array > 0);
            gl::GenBuffers(1, &mut index_buffer);
            assert!(index_buffer > 0);
            gl::GenBuffers(1, &mut vertex_buffer);
            assert!(vert_shader > 0);

            let (width, height) = window.size();
            let pixels_per_point = scale;
            let rect = vec2(width as f32, height as f32) / pixels_per_point;
            let screen_rect = Rect::from_min_size(Pos2::new(0f32, 0f32), rect);

            gl::DetachShader(program, vert_shader);
            gl::DetachShader(program, frag_shader);
            gl::DeleteShader(vert_shader);
            gl::DeleteShader(frag_shader);
            Painter {
                vertex_array,
                program,
                index_buffer,
                vertex_buffer,
                gl_sync_fence: gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0),
                pixels_per_point,
                textures: Default::default(),
                canvas_size: (width, height),
                screen_rect,
            }
        }
    }

    pub fn update_screen_rect(&mut self, size: (u32, u32)) {
        self.canvas_size = size;
        let (x, y) = size;
        let rect = vec2(x as f32, y as f32) / self.pixels_per_point;
        self.screen_rect = Rect::from_min_size(Default::default(), rect);
    }

    pub fn new_user_texture(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[Color32],
        filtering: bool,
    ) -> egui::TextureId {
        assert_eq!(size.0 * size.1, srgba_pixels.len());

        let mut pixels: Vec<u8> = Vec::with_capacity(srgba_pixels.len() * 4);
        for srgba in srgba_pixels {
            pixels.push(srgba[0]);
            pixels.push(srgba[1]);
            pixels.push(srgba[2]);
            pixels.push(srgba[3]);
        }

        let id = egui::TextureId::User(self.textures.len() as u64);
        self.textures.insert(
            id,
            Texture {
                size,
                pixels,
                gl_id: None,
                filtering,
                dirty: true,
            },
        );

        id
    }

    /// Creates a new user texture from rgba8
    pub fn new_user_texture_rgba8(
        &mut self,
        size: (usize, usize),
        rgba8_pixels: Vec<u8>,
        filtering: bool,
    ) -> egui::TextureId {
        let id = egui::TextureId::User(self.textures.len() as u64);
        self.textures.insert(
            id,
            Texture {
                size,
                pixels: rgba8_pixels,
                gl_id: None,
                filtering,
                dirty: true,
            },
        );

        id
    }

    /// fn free_texture() and fn free() implemented from epi both are basically the same.
    pub fn free_texture(&mut self, id: egui::TextureId) {
        if let Some(Texture {
            gl_id: Some(texture_id),
            ..
        }) = self.textures.get(&id)
        {
            unsafe { gl::DeleteTextures(1, texture_id) }
            self.textures.remove(&id);
        }
    }

    pub fn update_user_texture_data(&mut self, id: egui::TextureId, _pixels: &[Color32]) {
        if let Some(Texture { pixels, dirty, .. }) = self.textures.get_mut(&id) {
            *pixels = Vec::with_capacity(pixels.len() * 4);

            for p in _pixels {
                pixels.push(p[0]);
                pixels.push(p[1]);
                pixels.push(p[2]);
                pixels.push(p[3]);
            }

            *dirty = true;
        }
    }

    /// Updates texture rgba8 data
    pub fn update_user_texture_rgba8_data(&mut self, id: egui::TextureId, rgba8_pixels: Vec<u8>) {
        if let Some(Texture { pixels, dirty, .. }) = self.textures.get_mut(&id) {
            *pixels = rgba8_pixels;
            *dirty = true
        };
    }

    pub fn paint_jobs(
        &mut self,
        bg_color: Option<Color32>,
        textures_delta: egui::TexturesDelta,
        primitives: Vec<ClippedPrimitive>,
    ) {
        unsafe {
            gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
        }

        for (texture_id, delta) in textures_delta.set {
            self.upload_egui_texture(texture_id, &delta);
        }

        self.upload_user_textures();

        let (canvas_width, canvas_height) = self.canvas_size;
        let pixels_per_point = self.pixels_per_point;
        unsafe {
            if let Some(color) = bg_color {
                gl::ClearColor(
                    color[0] as f32 / 255.0,
                    color[1] as f32 / 255.0,
                    color[2] as f32 / 255.0,
                    color[3] as f32 / 255.0,
                );

                gl::Clear(gl::COLOR_BUFFER_BIT);
            }
            //Let OpenGL know we are dealing with SRGB colors so that it
            //can do the blending correctly. Not setting the framebuffer
            //leads to darkened, oversaturated colors.
            gl::Enable(gl::FRAMEBUFFER_SRGB);
            gl::Enable(gl::SCISSOR_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA); // premultiplied alpha
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);

            let u_screen_size = CString::new("u_screen_size").unwrap();
            let u_screen_size_ptr = u_screen_size.as_ptr();
            let u_screen_size_loc = gl::GetUniformLocation(self.program, u_screen_size_ptr);

            let (x, y) = (self.screen_rect.width(), self.screen_rect.height());
            gl::Uniform2f(u_screen_size_loc, x, y);

            let u_sampler = CString::new("u_sampler").unwrap();
            let u_sampler_ptr = u_sampler.as_ptr();
            let u_sampler_loc = gl::GetUniformLocation(self.program, u_sampler_ptr);
            gl::Uniform1i(u_sampler_loc, 0);
            gl::Viewport(0, 0, canvas_width as i32, canvas_height as i32);

            let screen_x = canvas_width as f32;
            let screen_y = canvas_height as f32;

            for ClippedPrimitive {
                clip_rect,
                primitive,
            } in primitives
            {
                match primitive {
                    Primitive::Mesh(mesh) => {
                        if let Some(Texture {
                            gl_id: Some(texture_gl_id),
                            ..
                        }) = self.textures.get(&mesh.texture_id)
                        {
                            {
                                gl::BindTexture(gl::TEXTURE_2D, *texture_gl_id);

                                let clip_min_x = pixels_per_point * clip_rect.min.x;
                                let clip_min_y = pixels_per_point * clip_rect.min.y;
                                let clip_max_x = pixels_per_point * clip_rect.max.x;
                                let clip_max_y = pixels_per_point * clip_rect.max.y;
                                let clip_min_x = clip_min_x.clamp(0.0, x);
                                let clip_min_y = clip_min_y.clamp(0.0, y);
                                let clip_max_x = clip_max_x.clamp(clip_min_x, screen_x);
                                let clip_max_y = clip_max_y.clamp(clip_min_y, screen_y);
                                let clip_min_x = clip_min_x.round() as i32;
                                let clip_min_y = clip_min_y.round() as i32;
                                let clip_max_x = clip_max_x.round() as i32;
                                let clip_max_y = clip_max_y.round() as i32;

                                //scissor Y coordinate is from the bottom
                                gl::Scissor(
                                    clip_min_x,
                                    canvas_height as i32 - clip_max_y,
                                    clip_max_x - clip_min_x,
                                    clip_max_y - clip_min_y,
                                );

                                self.paint_mesh(&mesh);
                            }
                        }
                    }
                    Primitive::Callback(_) => panic!("custom rendering not yet supported"),
                }
            }

            gl::Disable(gl::SCISSOR_TEST);
            gl::Disable(gl::FRAMEBUFFER_SRGB);
            gl::Disable(gl::BLEND);
        }

        for texture_id in textures_delta.free {
            self.free_texture(texture_id);
        }
    }

    pub fn cleanup(&self) {
        unsafe {
            gl::DeleteSync(self.gl_sync_fence);
            for (_, texture) in self.textures.iter() {
                if let Some(texture_gl_id) = texture.gl_id {
                    gl::DeleteTextures(1, &texture_gl_id);
                }
            }

            gl::DeleteProgram(self.program);
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteBuffers(1, &self.index_buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
        }
    }

    fn upload_egui_texture(&mut self, id: egui::TextureId, delta: &egui::epaint::ImageDelta) {
        // Modelled after egui_glium's set_texture().
        // From: https://github.com/emilk/egui/blob/
        //               34e6e12f002e7b477a8e8af6032097b00b96deea/crates/egui_glium/src/painter.rs
        let pixels: Vec<u8> = match &delta.image {
            egui::ImageData::Color(image) => {
                assert_eq!(
                    image.width() * image.height(),
                    image.pixels.len(),
                    "Mismatch between texture size and texel count"
                );

                image
                    .pixels
                    .iter()
                    .flat_map(|colour| colour.to_array())
                    .collect()
            }
            egui::ImageData::Font(image) => image
                .srgba_pixels(None)
                .flat_map(|colour| colour.to_array())
                .collect(),
        };
        let texture_width = delta.image.width();
        let texture_height = delta.image.height();

        if let Some(patch_pos) = delta.pos {
            if let Some(texture) = self.textures.get_mut(&id) {
                let patch_x = patch_pos[0];
                let patch_y = patch_pos[1];
                let patch_width = texture_width;
                let patch_height = texture_height;
                if texture.gl_id.is_some() {
                    unsafe {
                        let mipmap_level = 0;
                        let internal_format = gl::RGBA;
                        let texture_type = gl::UNSIGNED_BYTE;

                        gl::TexSubImage2D(
                            texture.gl_id.unwrap(),
                            mipmap_level,
                            patch_x as i32,
                            patch_y as i32,
                            patch_width as i32,
                            patch_height as i32,
                            internal_format,
                            texture_type,
                            pixels.as_ptr() as *const gl::types::GLvoid,
                        );
                    }
                }
            }
        } else {
            let texture_filtering: bool = true;
            let mut texture_gl_id = Option::None;
            Self::generate_gl_texture2d(
                &mut texture_gl_id,
                &pixels,
                texture_width as i32,
                texture_height as i32,
                texture_filtering,
            );

            self.textures.insert(
                id,
                Texture {
                    size: (texture_width, texture_height),
                    pixels,
                    gl_id: texture_gl_id,
                    filtering: true,
                    dirty: false,
                },
            );
        }
    }

    fn upload_user_textures(&mut self) {
        for (_, texture) in self.textures.iter_mut() {
            if !texture.dirty {
                continue;
            }

            let width = texture.size.0 as i32;
            let height = texture.size.1 as i32;
            let filtering = texture.filtering;
            let mut gl_id = texture.gl_id;
            Self::generate_gl_texture2d(&mut gl_id, &texture.pixels, width, height, filtering);

            texture.gl_id = gl_id;
            texture.dirty = false;
        }
    }

    fn paint_mesh(&self, mesh: &Mesh) {
        debug_assert!(mesh.is_valid());
        unsafe {
            let indices = &mesh.indices;
            let vertices = &mesh.vertices;

            // --------------------------------------------------------------------

            gl::BindVertexArray(self.vertex_array);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buffer);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u32>()) as GLsizeiptr,
                indices.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );

            // --------------------------------------------------------------------

            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<egui::epaint::Vertex>()) as GLsizeiptr,
                vertices.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );
            let stride: i32 = mem::size_of::<egui::epaint::Vertex>().try_into().unwrap();

            let a_pos = CString::new("a_pos").unwrap();
            let a_pos_ptr = a_pos.as_ptr();
            let a_pos_loc = gl::GetAttribLocation(self.program, a_pos_ptr);
            assert!(a_pos_loc >= 0);
            let a_pos_loc = a_pos_loc as u32;

            gl::VertexAttribPointer(
                a_pos_loc,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                memoffset::offset_of!(egui::epaint::Vertex, pos) as *const _,
            );
            gl::EnableVertexAttribArray(a_pos_loc);

            let a_tc = CString::new("a_tc").unwrap();
            let a_tc_ptr = a_tc.as_ptr();
            let a_tc_loc = gl::GetAttribLocation(self.program, a_tc_ptr);
            assert!(a_tc_loc >= 0);
            let a_tc_loc = a_tc_loc as u32;

            gl::VertexAttribPointer(
                a_tc_loc,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                memoffset::offset_of!(egui::epaint::Vertex, uv) as *const _,
            );
            gl::EnableVertexAttribArray(a_tc_loc);

            let a_srgba = CString::new("a_srgba").unwrap();
            let a_srgba_ptr = a_srgba.as_ptr();
            let a_srgba_loc = gl::GetAttribLocation(self.program, a_srgba_ptr);
            assert!(a_srgba_loc >= 0);
            let a_srgba_loc = a_srgba_loc as u32;

            gl::VertexAttribPointer(
                a_srgba_loc,
                4,
                gl::UNSIGNED_BYTE,
                gl::FALSE,
                stride,
                memoffset::offset_of!(egui::epaint::Vertex, color) as *const _,
            );
            gl::EnableVertexAttribArray(a_srgba_loc);

            // --------------------------------------------------------------------

            gl::DrawElements(
                gl::TRIANGLES,
                indices.len() as i32,
                gl::UNSIGNED_INT,
                ptr::null(),
            );
            gl::DisableVertexAttribArray(a_pos_loc);
            gl::DisableVertexAttribArray(a_tc_loc);
            gl::DisableVertexAttribArray(a_srgba_loc);
        }
    }

    fn generate_gl_texture2d(
        gl_id: &mut Option<GLuint>,
        pixels: &[u8],
        width: i32,
        height: i32,
        filtering: bool,
    ) {
        unsafe {
            if gl_id.is_none() {
                let mut texture_id = 0;
                gl::GenTextures(1, &mut texture_id);
                gl::BindTexture(gl::TEXTURE_2D, texture_id);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

                if filtering {
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                } else {
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                }

                *gl_id = Some(texture_id);
            } else {
                gl::BindTexture(gl::TEXTURE_2D, gl_id.unwrap());
            }

            let mipmap_level = 0;
            let internal_format = gl::RGBA;
            let border = 0;
            let src_format = gl::RGBA;
            let src_type = gl::UNSIGNED_BYTE;

            gl::TexImage2D(
                gl::TEXTURE_2D,
                mipmap_level,
                internal_format as i32,
                width,
                height,
                border,
                src_format,
                src_type,
                pixels.as_ptr() as *const gl::types::GLvoid,
            );
        }
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        self.cleanup();
    }
}
