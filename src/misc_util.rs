#![allow(unsafe_code)]

use gl::types::GLuint;

use crate::gl_utils::{create_shader, shader_source, get_shader_compile_status, get_shader_info_log, create_program, attach_shader, get_program_link_status, get_program_info_log};

pub(crate) unsafe fn compile_shader(
    shader_type: u32,
    source: &str,
) -> Result<GLuint, String> {
    let shader = create_shader(shader_type)?;

    shader_source(shader, source);

    gl::CompileShader(shader);

    if get_shader_compile_status(shader) {
        Ok(shader)
    } else {
        Err(get_shader_info_log(shader))
    }
}

pub(crate) unsafe fn link_program<'a, T: IntoIterator<Item = GLuint>>(
    shaders: T,
) -> Result<GLuint, String> {
    let program = create_program()?;

    for shader in shaders {
        attach_shader(program, shader);
    }

    gl::LinkProgram(program);

    if get_program_link_status(program) {
        Ok(program)
    } else {
        Err(get_program_info_log(program))
    }
}
