extern crate gl;
extern crate sdl2;
use gl::types::*;
use std::ffi::CString;
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use std::str;

use egui::{
    math::clamp,
    ClippedMesh,
    paint::{Color32, Texture, Mesh},
    vec2,
};

#[derive(Default)]
struct UserTexture {
    size: (usize, usize),

    /// Pending upload (will be emptied later).
    pixels: Vec<u8>,

    /// Lazily uploaded
    texture: Option<GLuint>,

    /// For user textures there is a choice between
    /// Linear (default) and Nearest.
    filtering: bool,

    /// User textures can be modified and this flag
    /// is used to indicate if pixel data for the
    /// texture has been updated.
    dirty: bool,
}

const VS_SRC: &str = r#"
    #version 150
    uniform vec2 u_screen_size;
    in vec2 a_pos;
    in vec4 a_srgba; // 0-255 sRGB
    in vec2 a_tc;
    out vec4 v_rgba;
    out vec2 v_tc;

    // 0-1 linear  from  0-255 sRGB
    vec3 linear_from_srgb(vec3 srgb) {
        bvec3 cutoff = lessThan(srgb, vec3(10.31475));
        vec3 lower = srgb / vec3(3294.6);
        vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
        return mix(higher, lower, cutoff);
    }

    vec4 linear_from_srgba(vec4 srgba) {
        return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
    }

    void main() {
        gl_Position = vec4(
            2.0 * a_pos.x / u_screen_size.x - 1.0,
            1.0 - 2.0 * a_pos.y / u_screen_size.y,
            0.0,
            1.0);
        v_rgba = linear_from_srgba(a_srgba);
        v_tc = a_tc;
    }
"#;

const FS_SRC: &str = r#"
    #version 150
    uniform sampler2D u_sampler;
    in vec4 v_rgba;
    in vec2 v_tc;
    out vec4 f_color;

    // 0-255 sRGB  from  0-1 linear
    vec3 srgb_from_linear(vec3 rgb) {
        bvec3 cutoff = lessThan(rgb, vec3(0.0031308));
        vec3 lower = rgb * vec3(3294.6);
        vec3 higher = vec3(269.025) * pow(rgb, vec3(1.0 / 2.4)) - vec3(14.025);
        return mix(higher, lower, vec3(cutoff));
    }

    vec4 srgba_from_linear(vec4 rgba) {
        return vec4(srgb_from_linear(rgba.rgb), 255.0 * rgba.a);
    }
    
    vec3 linear_from_srgb(vec3 srgb) {
        bvec3 cutoff = lessThan(srgb, vec3(10.31475));
        vec3 lower = srgb / vec3(3294.6);
        vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
        return mix(higher, lower, vec3(cutoff));
    }

    vec4 linear_from_srgba(vec4 srgba) {
        return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
    }

    void main() {
        // Need to convert from SRGBA to linear.
        vec4 texture_rgba = linear_from_srgba(texture(u_sampler, v_tc) * 255.0);
        f_color = srgba_from_linear(v_rgba * texture_rgba) / 255.0;

        // If the gl implementation doesn't require conversion:
        //f_color = v_rgba * texture(u_sampler, v_tc);
    }
"#;

pub struct Painter {
    program: GLuint,
    index_buffer: GLuint,
    pos_buffer: GLuint,
    tc_buffer: GLuint,
    color_buffer: GLuint,
    canvas_width: u32,
    canvas_height: u32,
    egui_texture: GLuint,
    egui_texture_version: Option<u64>,
    vert_shader: GLuint,
    frag_shader: GLuint,
    user_textures: Vec<UserTexture>,
}

fn compile_shader(src: &str, ty: GLenum) -> GLuint {
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
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf).expect("ShaderInfoLog not valid utf8")
            );
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
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
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            panic!(
                "{}",
                str::from_utf8(&buf).expect("ProgramInfoLog not valid utf8")
            );
        }
        program
    }
}

impl Painter {
    pub fn new(
        video_subsystem: &sdl2::VideoSubsystem,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Painter {
        unsafe {
            let mut egui_texture = 0;
            gl::load_with(|name| video_subsystem.gl_get_proc_address(name) as *const _);
            gl::GenTextures(1, &mut egui_texture);
            gl::BindTexture(gl::TEXTURE_2D, egui_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            let vert_shader = compile_shader(VS_SRC, gl::VERTEX_SHADER);
            let frag_shader = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);

            let program = link_program(vert_shader, frag_shader);
            let mut index_buffer = 0;
            let mut pos_buffer = 0;
            let mut tc_buffer = 0;
            let mut color_buffer = 0;
            gl::GenVertexArrays(1, &mut index_buffer);
            gl::BindVertexArray(index_buffer);
            gl::GenBuffers(1, &mut index_buffer);
            gl::GenBuffers(1, &mut pos_buffer);
            gl::GenBuffers(1, &mut tc_buffer);
            gl::GenBuffers(1, &mut color_buffer);

            Painter {
                program,
                canvas_width,
                canvas_height,
                index_buffer,
                pos_buffer,
                tc_buffer,
                color_buffer,
                egui_texture,
                vert_shader,
                frag_shader,
                egui_texture_version: None,
                user_textures: Default::default(),
            }
        }
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
            pixels.push(srgba.r());
            pixels.push(srgba.g());
            pixels.push(srgba.b());
            pixels.push(srgba.a());
        }

        let id = egui::TextureId::User(self.user_textures.len() as u64);
        self.user_textures.push(UserTexture {
            size,
            pixels,
            texture: None,
            filtering,
            dirty: true,
        });
        id
    }

    fn upload_egui_texture(&mut self, texture: &Texture) {
        if self.egui_texture_version == Some(texture.version) {
            return; // No change
        }

        let mut pixels: Vec<u8> = Vec::with_capacity(texture.pixels.len() * 4);
        for &alpha in &texture.pixels {
            let srgba = Color32::from_white_alpha(alpha);
            pixels.push(srgba.r());
            pixels.push(srgba.g());
            pixels.push(srgba.b());
            pixels.push(srgba.a());
        }

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.egui_texture);

            let level = 0;
            let internal_format = gl::RGBA;
            let border = 0;
            let src_format = gl::RGBA;
            let src_type = gl::UNSIGNED_BYTE;
            gl::TexImage2D(
                gl::TEXTURE_2D,
                level,
                internal_format as i32,
                texture.width as i32,
                texture.height as i32,
                border,
                src_format,
                src_type,
                pixels.as_ptr() as *const c_void,
            );

            self.egui_texture_version = Some(texture.version);
        }
    }

    fn upload_user_textures(&mut self) {
        unsafe {
            for user_texture in &mut self.user_textures {
                if !user_texture.texture.is_none() && !user_texture.dirty {
                    continue;
                }
                let pixels = std::mem::take(&mut user_texture.pixels);

                if user_texture.texture.is_none() {
                    let mut gl_texture = 0;
                    gl::GenTextures(1, &mut gl_texture);
                    gl::BindTexture(gl::TEXTURE_2D, gl_texture);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

                    if user_texture.filtering {
                        gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MIN_FILTER,
                            gl::LINEAR as i32,
                        );
                        gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MAG_FILTER,
                            gl::LINEAR as i32,
                        );
                    } else {
                        gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MIN_FILTER,
                            gl::NEAREST as i32,
                        );
                        gl::TexParameteri(
                            gl::TEXTURE_2D,
                            gl::TEXTURE_MAG_FILTER,
                            gl::NEAREST as i32,
                        );
                    }
                    user_texture.texture = Some(gl_texture);
                } else {
                    gl::BindTexture(gl::TEXTURE_2D, user_texture.texture.unwrap());
                }

                let level = 0;
                let internal_format = gl::RGBA;
                let border = 0;
                let src_format = gl::RGBA;
                let src_type = gl::UNSIGNED_BYTE;

                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    level,
                    internal_format as i32,
                    user_texture.size.0 as i32,
                    user_texture.size.1 as i32,
                    border,
                    src_format,
                    src_type,
                    pixels.as_ptr() as *const c_void,
                );

                user_texture.dirty = false;
            }
        }
    }

    fn get_texture(&self, texture_id: egui::TextureId) -> GLuint {
        match texture_id {
            egui::TextureId::Egui => self.egui_texture,
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                let texture = self.user_textures[id].texture;
                texture.expect("Should have been uploaded")
            }
        }
    }

    pub fn update_user_texture_data(&mut self, texture_id: egui::TextureId, pixels: &[Color32]) {
        match texture_id {
            egui::TextureId::Egui => {}
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                self.user_textures[id].pixels = Vec::with_capacity(pixels.len() * 4);

                for p in pixels {
                    self.user_textures[id].pixels.push(p.r());
                    self.user_textures[id].pixels.push(p.g());
                    self.user_textures[id].pixels.push(p.b());
                    self.user_textures[id].pixels.push(p.a());
                }
                self.user_textures[id].dirty = true;
            }
        }
    }

    pub fn paint_jobs(
        &mut self,
        bg_color: Color32,
        meshes: Vec<ClippedMesh>,
        egui_texture: &Texture,
        pixels_per_point: f32,
    ) {
        self.upload_egui_texture(egui_texture);
        self.upload_user_textures();

        unsafe {
            gl::ClearColor(
                bg_color[0] as f32 / 255.0,
                bg_color[1] as f32 / 255.0,
                bg_color[2] as f32 / 255.0,
                bg_color[3] as f32 / 255.0,
            );

            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::Enable(gl::SCISSOR_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA); // premultiplied alpha
            gl::UseProgram(self.program);
            gl::ActiveTexture(gl::TEXTURE0);

            let u_screen_size = CString::new("u_screen_size").unwrap();
            let u_screen_size_ptr = u_screen_size.as_ptr();
            let u_screen_size_loc = gl::GetUniformLocation(self.program, u_screen_size_ptr);
            let screen_size_pixels = vec2(self.canvas_width as f32, self.canvas_height as f32);
            let screen_size_points = screen_size_pixels / pixels_per_point;
            gl::Uniform2f(
                u_screen_size_loc,
                screen_size_points.x,
                screen_size_points.y,
            );
            let u_sampler = CString::new("u_sampler").unwrap();
            let u_sampler_ptr = u_sampler.as_ptr();
            let u_sampler_loc = gl::GetUniformLocation(self.program, u_sampler_ptr);
            gl::Uniform1i(u_sampler_loc, 0);
            gl::Viewport(0, 0, self.canvas_width as i32, self.canvas_height as i32);

            for ClippedMesh(clip_rect, mesh) in meshes {
                gl::BindTexture(gl::TEXTURE_2D, self.get_texture(mesh.texture_id));

                let clip_min_x = pixels_per_point * clip_rect.min.x;
                let clip_min_y = pixels_per_point * clip_rect.min.y;
                let clip_max_x = pixels_per_point * clip_rect.max.x;
                let clip_max_y = pixels_per_point * clip_rect.max.y;
                let clip_min_x = clamp(clip_min_x, 0.0..=screen_size_pixels.x);
                let clip_min_y = clamp(clip_min_y, 0.0..=screen_size_pixels.y);
                let clip_max_x = clamp(clip_max_x, clip_min_x..=screen_size_pixels.x);
                let clip_max_y = clamp(clip_max_y, clip_min_y..=screen_size_pixels.y);
                let clip_min_x = clip_min_x.round() as i32;
                let clip_min_y = clip_min_y.round() as i32;
                let clip_max_x = clip_max_x.round() as i32;
                let clip_max_y = clip_max_y.round() as i32;

                //scissor Y coordinate is from the bottom
                gl::Scissor(
                    clip_min_x,
                    self.canvas_height as i32 - clip_max_y,
                    clip_max_x - clip_min_x,
                    clip_max_y - clip_min_y,
                );

                self.paint_mesh(&mesh);
                gl::Disable(gl::SCISSOR_TEST);
            }
        }
    }

    pub fn cleanup(&self) {
        unsafe {
            gl::DeleteProgram(self.program);
            gl::DeleteShader(self.vert_shader);
            gl::DeleteShader(self.frag_shader);
            gl::DeleteBuffers(1, &self.pos_buffer);
            gl::DeleteBuffers(1, &self.tc_buffer);
            gl::DeleteBuffers(1, &self.color_buffer);
            gl::DeleteVertexArrays(1, &self.index_buffer);
        }
    }

    fn paint_mesh(&self, mesh: &Mesh) {
        debug_assert!(mesh.is_valid());
        let indices: Vec<u16> = mesh.indices.iter().map(|idx| *idx as u16).collect();

        let mut positions: Vec<f32> = Vec::with_capacity(2 * mesh.vertices.len());
        let mut tex_coords: Vec<f32> = Vec::with_capacity(2 * mesh.vertices.len());
        for v in &mesh.vertices {
            positions.push(v.pos.x);
            positions.push(v.pos.y);
            tex_coords.push(v.uv.x);
            tex_coords.push(v.uv.y);
        }

        let mut colors: Vec<u8> = Vec::with_capacity(4 * mesh.vertices.len());
        for v in &mesh.vertices {
            colors.push(v.color[0]);
            colors.push(v.color[1]);
            colors.push(v.color[2]);
            colors.push(v.color[3]);
        }

        unsafe {
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buffer);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<u16>()) as GLsizeiptr,
                //mem::transmute(&indices.as_ptr()),
                indices.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );

            // --------------------------------------------------------------------
            gl::BindBuffer(gl::ARRAY_BUFFER, self.pos_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (positions.len() * mem::size_of::<f32>()) as GLsizeiptr,
                //mem::transmute(&positions.as_ptr()),
                positions.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );

            let a_pos = CString::new("a_pos").unwrap();
            let a_pos_ptr = a_pos.as_ptr();
            let a_pos_loc = gl::GetAttribLocation(self.program, a_pos_ptr);
            assert!(a_pos_loc >= 0);
            let a_pos_loc = a_pos_loc as u32;

            let stride = 0;
            gl::VertexAttribPointer(a_pos_loc, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::EnableVertexAttribArray(a_pos_loc);

            // --------------------------------------------------------------------

            gl::BindBuffer(gl::ARRAY_BUFFER, self.tc_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (tex_coords.len() * mem::size_of::<f32>()) as GLsizeiptr,
                //mem::transmute(&tex_coords.as_ptr()),
                tex_coords.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );

            let a_tc = CString::new("a_tc").unwrap();
            let a_tc_ptr = a_tc.as_ptr();
            let a_tc_loc = gl::GetAttribLocation(self.program, a_tc_ptr);
            assert!(a_tc_loc >= 0);
            let a_tc_loc = a_tc_loc as u32;

            let stride = 0;
            gl::VertexAttribPointer(a_tc_loc, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::EnableVertexAttribArray(a_tc_loc);

            // --------------------------------------------------------------------
            gl::BindBuffer(gl::ARRAY_BUFFER, self.color_buffer);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (colors.len() * mem::size_of::<u8>()) as GLsizeiptr,
                //mem::transmute(&colors.as_ptr()),
                colors.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );

            let a_srgba = CString::new("a_srgba").unwrap();
            let a_srgba_ptr = a_srgba.as_ptr();
            let a_srgba_loc = gl::GetAttribLocation(self.program, a_srgba_ptr);
            assert!(a_srgba_loc >= 0);
            let a_srgba_loc = a_srgba_loc as u32;

            let stride = 0;
            gl::VertexAttribPointer(
                a_srgba_loc,
                4,
                gl::UNSIGNED_BYTE,
                gl::FALSE,
                stride,
                ptr::null(),
            );
            gl::EnableVertexAttribArray(a_srgba_loc);

            // --------------------------------------------------------------------

            gl::DrawElements(
                gl::TRIANGLES,
                indices.len() as i32,
                gl::UNSIGNED_SHORT,
                ptr::null(),
            );
        }
    }
}
