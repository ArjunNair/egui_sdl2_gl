use std::ffi::CString;

use gl::{types::{GLchar, GLint, GLuint}, INFO_LOG_LENGTH, LINK_STATUS};

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

pub unsafe fn get_uniform_location(
    program: GLuint,
    name: &str,
) -> Option<GLuint> {
    let name = CString::new(name).unwrap();
    let uniform_location =
        gl::GetUniformLocation(program, name.as_ptr() as *const GLchar);
    if uniform_location < 0 {
        None
    } else {
        Some(uniform_location as GLuint)
    }
}

pub unsafe fn create_buffer() -> Result<GLuint, String> {
    let mut buffer = 0;
    gl::GenBuffers(1, &mut buffer);
    Ok(buffer)
}

pub unsafe fn get_attrib_location(program: GLuint, name: &str) -> Option<u32> {
    let name = CString::new(name).unwrap();
    let attrib_location =
        gl::GetAttribLocation(program, name.as_ptr() as *const GLchar);
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


