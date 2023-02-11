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
