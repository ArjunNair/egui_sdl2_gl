use crate::shader_version;
use gl::{
    types::{GLchar, GLint, GLuint},
    INFO_LOG_LENGTH, LINK_STATUS,
};
pub use shader_version::ShaderVersion;
use std::ffi::CString;

/// Check for OpenGL error and report it using `tracing::error`.
///
/// Only active in debug builds!
///
#[macro_export]
macro_rules! check_for_gl_error {
    () => {{
        if cfg!(debug_assertions) {
            $crate::gl::check_for_gl_error_impl(file!(), line!(), "")
        }
    }};
    ($context: literal) => {{
        if cfg!(debug_assertions) {
            $crate::gl::check_for_gl_error_impl(file!(), line!(), $context)
        }
    }};
}

/// Check for OpenGL error and report it using `tracing::error`.
///
/// WARNING: slow! Only use during setup!
///
/// ``` no_run
/// # let glow_context = todo!();
/// use egui_glow::check_for_gl_error_even_in_release;
/// check_for_gl_error_even_in_release!(glow_context);
/// check_for_gl_error_even_in_release!(glow_context, "during painting");
/// ```
#[macro_export]
macro_rules! check_for_gl_error_even_in_release {
    () => {{
        $crate::gl::check_for_gl_error_impl(file!(), line!(), "")
    }};
    ($context: literal) => {{
        $crate::gl::check_for_gl_error_impl(file!(), line!(), $context)
    }};
}

#[doc(hidden)]
pub fn check_for_gl_error_impl(file: &str, line: u32, context: &str) {
    #[allow(unsafe_code)]
    let error_code = unsafe { gl::GetError() };
    if error_code != gl::NO_ERROR {
        let error_str = match error_code {
            gl::INVALID_ENUM => "GL_INVALID_ENUM",
            gl::INVALID_VALUE => "GL_INVALID_VALUE",
            gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
            gl::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
            gl::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
            gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
            gl::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
            gl::CONTEXT_LOST => "GL_CONTEXT_LOST",
            0x8031 => "GL_TABLE_TOO_LARGE1",
            0x9242 => "CONTEXT_LOST_WEBGL",
            _ => "<unknown>",
        };

        if context.is_empty() {
            tracing::error!(
                "GL error, at {}:{}: {} (0x{:X}). Please file a bug at https://github.com/emilk/egui/issues",
                file,
                line,
                error_str,
                error_code,
            );
        } else {
            tracing::error!(
                "GL error, at {}:{} ({}): {} (0x{:X}). Please file a bug at https://github.com/emilk/egui/issues",
                file,
                line,
                context,
                error_str,
                error_code,
            );
        }
    }
}

pub unsafe fn get_parameter_string(parameter: u32) -> String {
    let raw_ptr = gl::GetString(parameter);
    if raw_ptr.is_null() {
        panic!(
            "Get parameter string 0x{:X} failed. Maybe your GL context version is too outdated.",
            parameter
        )
    }
    std::ffi::CStr::from_ptr(raw_ptr as *const GLchar)
        .to_str()
        .unwrap()
        .to_owned()
}

pub unsafe fn get_parameter_i32(parameter: u32) -> i32 {
    let mut value = 0;
    gl::GetIntegerv(parameter, &mut value);
    value
}

pub unsafe fn create_shader(shader_type: u32) -> Result<GLuint, String> {
    Ok(gl::CreateShader(shader_type as u32)) // TODO: check errors
}

pub unsafe fn shader_source(shader: GLuint, source: &str) {
    gl::ShaderSource(
        shader,
        1,
        &(source.as_ptr() as *const GLchar),
        &(source.len() as GLint),
    );
}

pub unsafe fn get_shader_compile_status(shader: GLuint) -> bool {
    let mut status = 0;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    1 == status
}

pub unsafe fn get_shader_info_log(shader: GLuint) -> String {
    let mut length = 0;
    gl::GetShaderiv(shader, INFO_LOG_LENGTH, &mut length);
    if length > 0 {
        let mut log = String::with_capacity(length as usize);
        log.extend(std::iter::repeat('\0').take(length as usize));
        gl::GetShaderInfoLog(
            shader,
            length,
            &mut length,
            (&log[..]).as_ptr() as *mut GLchar,
        );
        log.truncate(length as usize);
        log
    } else {
        String::from("")
    }
}

pub unsafe fn create_program() -> Result<GLuint, String> {
    Ok(gl::CreateProgram()) // TODO: error check
}

pub unsafe fn attach_shader(program: GLuint, shader: GLuint) {
    gl::AttachShader(program, shader); // TODO: error check
}

pub unsafe fn get_program_link_status(program: GLuint) -> bool {
    let mut status = 0;
    gl::GetProgramiv(program, LINK_STATUS, &mut status);
    1 == status
}

pub unsafe fn get_program_info_log(program: GLuint) -> String {
    let mut length = 0;
    gl::GetProgramiv(program, INFO_LOG_LENGTH, &mut length);
    if length > 0 {
        let mut log = String::with_capacity(length as usize);
        log.extend(std::iter::repeat('\0').take(length as usize));
        gl::GetProgramInfoLog(
            program,
            length,
            &mut length,
            (&log[..]).as_ptr() as *mut GLchar,
        );
        log.truncate(length as usize);
        log
    } else {
        String::from("")
    }
}

pub unsafe fn get_uniform_location(program: GLuint, name: &str) -> Option<GLint> {
    let name = CString::new(name).unwrap();
    let uniform_location = gl::GetUniformLocation(program, name.as_ptr() as *const GLchar);
    if uniform_location < 0 {
        None
    } else {
        Some(uniform_location)
    }
}

pub unsafe fn create_buffer() -> Result<GLuint, String> {
    let mut buffer = 0;
    gl::GenBuffers(1, &mut buffer);
    Ok(buffer)
}

pub unsafe fn get_attrib_location(program: GLuint, name: &str) -> Option<u32> {
    let name = CString::new(name).unwrap();
    let attrib_location = gl::GetAttribLocation(program, name.as_ptr() as *const GLchar);
    if attrib_location < 0 {
        None
    } else {
        Some(attrib_location as u32)
    }
}

pub unsafe fn create_vertex_array() -> Result<GLuint, String> {
    let mut vertex_array = 0;
    gl::GenVertexArrays(1, &mut vertex_array);
    Ok(vertex_array)
}

pub unsafe fn vertex_attrib_pointer_f32(
    index: u32,
    size: i32,
    data_type: u32,
    normalized: bool,
    stride: i32,
    offset: i32,
) {
    gl::VertexAttribPointer(
        index,
        size,
        data_type,
        normalized as u8,
        stride,
        offset as *const std::ffi::c_void,
    );
}

pub unsafe fn color_mask(red: bool, green: bool, blue: bool, alpha: bool) {
    gl::ColorMask(red as u8, green as u8, blue as u8, alpha as u8);
}

pub unsafe fn blend_equation_separate(mode_rgb: u32, mode_alpha: u32) {
    gl::BlendEquationSeparate(mode_rgb as u32, mode_alpha as u32);
}

pub unsafe fn blend_func_separate(src_rgb: u32, dst_rgb: u32, src_alpha: u32, dst_alpha: u32) {
    gl::BlendFuncSeparate(
        src_rgb as u32,
        dst_rgb as u32,
        src_alpha as u32,
        dst_alpha as u32,
    );
}

// pub unsafe fn bind_vertex_array(vao: GLuint) {
//     gl::BindVertexArray(vao);
//     check_for_gl_error!("bind_vertex_array");
// }

pub unsafe fn buffer_data_u8_slice(target: u32, data: &[u8], usage: u32) {
    gl::BufferData(
        target,
        data.len() as isize,
        data.as_ptr() as *const std::ffi::c_void,
        usage,
    );
}

pub unsafe fn create_texture() -> Result<GLuint, String> {
    let mut name = 0;
    gl::GenTextures(1, &mut name);
    Ok(name)
}
