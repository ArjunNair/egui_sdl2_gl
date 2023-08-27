extern crate gl;
extern crate sdl2;
use crate::ShaderVersion;
use core::mem;
use core::ptr;
use core::str;
use ahash::AHashMap;
use egui::{
    epaint::{Color32, FontImage, Mesh, Primitive},
    vec2, ClippedPrimitive, Pos2, Rect,
};
use gl::types::{GLchar, GLenum, GLint, GLsizeiptr, GLsync, GLuint};
use std::ffi::CString;
use std::os::raw::c_void;

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

const VS_SRC_150: &str = r#"
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

const FS_SRC_150: &str = r#"
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
        f_color = v_rgba * texture_rgba;
    }
"#;

// VS_SRC and FS_SRC shaders taken from egui_glow crate.
const VS_SRC: &str = r#"
#if !defined(GL_ES) && __VERSION__ >= 140
#define I in
#define O out
#define V(x) x
#else
#define I attribute
#define O varying
#define V(x) vec3(x)
#endif

#ifdef GL_ES
precision mediump float;
#endif
uniform vec2 u_screen_size;
I vec2 a_pos;
I vec4 a_srgba; // 0-255 sRGB
I vec2 a_tc;
O vec4 v_rgba;
O vec2 v_tc;

// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
  bvec3 cutoff = lessThan(srgb, vec3(10.31475));
  vec3 lower = srgb / vec3(3294.6);
  vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
  return mix(higher, lower, V(cutoff));
}

vec4 linear_from_srgba(vec4 srgba) {
  return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
}

void main() {
  gl_Position = vec4(2.0 * a_pos.x / u_screen_size.x - 1.0, 1.0 - 2.0 * a_pos.y / u_screen_size.y, 0.0, 1.0);
  // egui encodes vertex colors in gamma spaces, so we must decode the colors here:
  v_rgba = linear_from_srgba(a_srgba);
  v_tc = a_tc;
}
"#;

const FS_SRC: &str = r#"
#ifdef GL_ES
precision mediump float;
#endif

uniform sampler2D u_sampler;
#if defined(GL_ES) || __VERSION__ < 140
varying vec4 v_rgba;
varying vec2 v_tc;
#else
in vec4 v_rgba;
in vec2 v_tc;
out vec4 f_color;
#endif

#ifdef GL_ES
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

#if __VERSION__ < 300
// 0-1 linear  from  0-255 sRGB
vec3 linear_from_srgb(vec3 srgb) {
  bvec3 cutoff = lessThan(srgb, vec3(10.31475));
  vec3 lower = srgb / vec3(3294.6);
  vec3 higher = pow((srgb + vec3(14.025)) / vec3(269.025), vec3(2.4));
  return mix(higher, lower, vec3(cutoff));
}

vec4 linear_from_srgba(vec4 srgba) {
  return vec4(linear_from_srgb(srgba.rgb), srgba.a / 255.0);
}
#endif
#endif

#ifdef GL_ES
void main() {
#if __VERSION__ < 300
  // We must decode the colors, since WebGL doesn't come with sRGBA textures:
  vec4 texture_rgba = linear_from_srgba(texture2D(u_sampler, v_tc) * 255.0);
#else
  // The texture is set up with `SRGB8_ALPHA8`, so no need to decode here!
  vec4 texture_rgba = texture2D(u_sampler, v_tc);
#endif

  /// Multiply vertex color with texture color (in linear space).
  gl_FragColor = v_rgba * texture_rgba;

  // We must gamma-encode again since WebGL doesn't support linear blending in the framebuffer.
  gl_FragColor = srgba_from_linear(v_rgba * texture_rgba) / 255.0;

  // WebGL doesn't support linear blending in the framebuffer,
  // so we apply this hack to at least get a bit closer to the desired blending:
  gl_FragColor.a = pow(gl_FragColor.a, 1.6); // Empiric nonsense
}
#else
void main() {
  // The texture sampler is sRGB aware, and OpenGL already expects linear rgba output
  // so no need for any sRGB conversions here:
#if __VERSION__ < 140
  gl_FragColor = v_rgba * texture2D(u_sampler, v_tc);
#else
  f_color = v_rgba * texture(u_sampler, v_tc);
#endif
}
#endif
"#;

pub struct Painter {
    vertex_array: GLuint,
    program: GLuint,
    index_buffer: GLuint,
    pos_buffer: GLuint,
    tc_buffer: GLuint,
    color_buffer: GLuint,
    // Call fence for sdl vsync so the CPU won't heat up if there's no heavy activity.
    pub gl_sync_fence: GLsync,
    user_textures: AHashMap<egui::TextureId, UserTexture>,
    pub pixels_per_point: f32,
    pub canvas_size: (u32, u32),
    pub screen_rect: Rect,
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
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            // clippy not happy with this, broke the CI:
            // error: calling `set_len()` immediately after reserving a buffer creates uninitialized values
            // buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
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
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            // clippy not happy with this, broke the CI:
            // error: calling `set_len()` immediately after reserving a buffer creates uninitialized values
            // buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
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
    pub fn new(window: &sdl2::video::Window, scale: f32, shader_ver: ShaderVersion) -> Painter {
        unsafe {
            gl::load_with(|name| window.subsystem().gl_get_proc_address(name) as *const _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            let (vs_src, fs_src) = if let ShaderVersion::Default = shader_ver {
                (VS_SRC_150, FS_SRC_150)
            } else {
                (VS_SRC, FS_SRC)
            };
            let vert_shader = compile_shader(vs_src, gl::VERTEX_SHADER);
            let frag_shader = compile_shader(fs_src, gl::FRAGMENT_SHADER);

            let program = link_program(vert_shader, frag_shader);
            let mut vertex_array = 0;
            let mut index_buffer = 0;
            let mut pos_buffer = 0;
            let mut tc_buffer = 0;
            let mut color_buffer = 0;
            gl::GenVertexArrays(1, &mut vertex_array);
            gl::BindVertexArray(vertex_array);
            gl::GenBuffers(1, &mut index_buffer);
            gl::GenBuffers(1, &mut pos_buffer);
            gl::GenBuffers(1, &mut tc_buffer);
            gl::GenBuffers(1, &mut color_buffer);

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
                pos_buffer,
                tc_buffer,
                color_buffer,
                gl_sync_fence: gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0),
                pixels_per_point,
                user_textures: Default::default(),
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

        let id = egui::TextureId::User(self.user_textures.len() as u64);
        self.user_textures.insert(id, UserTexture {
            size,
            pixels,
            texture: None,
            filtering,
            dirty: true,
        });

        id
    }

    /// Creates a new user texture from rgba8
    pub fn new_user_texture_rgba8(
        &mut self,
        size: (usize, usize),
        rgba8_pixels: Vec<u8>,
        filtering: bool,
    ) -> egui::TextureId {
        let id = egui::TextureId::User(self.user_textures.len() as u64);
        self.user_textures.insert(id, UserTexture {
            size,
            pixels: rgba8_pixels,
            texture: None,
            filtering,
            dirty: true,
        });

        id
    }

    /// fn free_user_texture() and fn free() implemented from epi both are basically the same.
    pub fn free_user_texture(&mut self, id: egui::TextureId) {
        if let Some(UserTexture { texture: Some(texture), .. }) = self.user_textures.get(&id)
        {
            unsafe { gl::DeleteTextures(1, texture) }
            self.user_textures.remove(&id);
        }
    }

    fn upload_user_textures(&mut self) {
        unsafe {
            for (id, user_texture) in self.user_textures.iter_mut() {
                if !user_texture.dirty {
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

    pub fn update_user_texture_data(&mut self, id: egui::TextureId, _pixels: &[Color32]) {
        if let Some(UserTexture { pixels, dirty, .. }) = self.user_textures.get_mut(&id) {
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
    pub fn update_user_texture_rgba8_data(
        &mut self,
        id: egui::TextureId,
        rgba8_pixels: Vec<u8>,
    ) {
        if let Some(UserTexture { pixels, dirty, .. }) = self.user_textures.get_mut(&id) {
            *pixels = rgba8_pixels;
            *dirty = true
        };
    }

    pub fn paint_jobs(
        &mut self,
        bg_color: Option<Color32>,
        primitives: Vec<ClippedPrimitive>
    ) {
        unsafe {
            gl::PixelStorei(gl::UNPACK_ROW_LENGTH, 0);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 4);
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

            for ClippedPrimitive{clip_rect, primitive} in primitives {
                match primitive {
                    Primitive::Mesh(mesh) => {
                        if let Some(UserTexture {
                            size,
                            pixels,
                            texture,
                            filtering,
                            dirty
                        }) = self.user_textures.get(&mesh.texture_id) {
                            if let Some(texture_gl_id) = texture {
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
                    },
                    Primitive::Callback(_) => panic!("custom rendering not yet supported")
                }
            }

            gl::Disable(gl::SCISSOR_TEST);
            gl::Disable(gl::FRAMEBUFFER_SRGB);
            gl::Disable(gl::BLEND);
        }
    }

    fn paint_mesh(&self, mesh: &Mesh) {
        debug_assert!(mesh.is_valid());
        unsafe {
            let indices: Vec<u16> = mesh.indices.iter().map(move |idx| *idx as u16).collect();
            let indices_len = indices.len();
            let vertices = &mesh.vertices;
            let vertices_len = vertices.len();

            // --------------------------------------------------------------------

            gl::BindVertexArray(self.vertex_array);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buffer);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (indices_len * mem::size_of::<u16>()) as GLsizeiptr,
                //mem::transmute(&indices.as_ptr()),
                indices.as_ptr() as *const gl::types::GLvoid,
                gl::STREAM_DRAW,
            );

            // --------------------------------------------------------------------

            let mut positions: Vec<f32> = Vec::with_capacity(2 * vertices_len);
            let mut tex_coords: Vec<f32> = Vec::with_capacity(2 * vertices_len);
            {
                for v in &mesh.vertices {
                    positions.push(v.pos.x);
                    positions.push(v.pos.y);
                    tex_coords.push(v.uv.x);
                    tex_coords.push(v.uv.y);
                }
            }

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

            let mut colors: Vec<u8> = Vec::with_capacity(4 * vertices_len);
            {
                for v in vertices {
                    colors.push(v.color[0]);
                    colors.push(v.color[1]);
                    colors.push(v.color[2]);
                    colors.push(v.color[3]);
                }
            }

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
                indices_len as i32,
                gl::UNSIGNED_SHORT,
                ptr::null(),
            );
            gl::DisableVertexAttribArray(a_pos_loc);
            gl::DisableVertexAttribArray(a_tc_loc);
            gl::DisableVertexAttribArray(a_srgba_loc);
        }
    }

    pub fn cleanup(&self) {
        unsafe {
            gl::DeleteSync(self.gl_sync_fence);
            for (_, user_texture) in self.user_textures.iter() {
                if let Some(texture_gl_id) = user_texture.texture {
                    gl::DeleteTextures(1, &texture_gl_id);
                }
            }

            gl::DeleteProgram(self.program);
            gl::DeleteBuffers(1, &self.pos_buffer);
            gl::DeleteBuffers(1, &self.tc_buffer);
            gl::DeleteBuffers(1, &self.color_buffer);
            gl::DeleteBuffers(1, &self.index_buffer);
            gl::DeleteVertexArrays(1, &self.vertex_array);
        }
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        self.cleanup();
    }
}
