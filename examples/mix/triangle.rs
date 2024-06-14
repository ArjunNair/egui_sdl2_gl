// Draws a simple colored triangle
// based on the example from:
// https://github.com/brendanzab/gl-rs/blob/master/gl/examples/triangle.rs
// and https://learnopengl.com/Getting-started/Shaders

use egui_sdl2_gl::gl;
use egui_sdl2_gl::gl::types::*;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::str;

const VS_SRC: &str = "
#version 150
in vec3 aPosition;
in vec3 aColor;

out vec3 color;

void main() {
    gl_Position = vec4(aPosition, 1.0);
    color = aColor;
}";

const FS_SRC: &str = "
#version 150
out vec4 out_color;
in vec3 color;

void main() {
    out_color = vec4(color, 1.0);
}";

#[rustfmt::skip]
static VERTEX_DATA: [GLfloat; 18] = [
    // position         //rgb color
    0.0, 0.5, 0.0,      0.0, 0.0, 1.0,
    0.5, -0.5, 0.0,     0.0, 1.0, 0.0,
    -0.5,-0.5, 0.0,     1.0, 0.0, 0.0
];

pub struct Triangle {
    pub program: GLuint,
    pub vao: GLuint,
    pub vbo: GLuint,
}

pub fn compile_shader(src: &str, ty: GLenum) -> GLuint {
    let shader;
    unsafe {
        // Create GLSL shaders
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

pub fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        gl::DetachShader(program, fs);
        gl::DetachShader(program, vs);
        gl::DeleteShader(fs);
        gl::DeleteShader(vs);

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

impl Triangle {
    pub fn new() -> Self {
        // Create Vertex Array Object
        let mut vao = 0;
        let mut vbo = 0;
        let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
        let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
        let program = link_program(vs, fs);
        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            // Create a VAO since the data is setup only once.
            gl::BindVertexArray(vao);

            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                mem::transmute(&VERTEX_DATA[0]),
                gl::STATIC_DRAW,
            );

            // Specify the layout of the vertex data
            let c_position = CString::new("aPosition").unwrap();
            let pos_attr = gl::GetAttribLocation(program, c_position.as_ptr());
            gl::EnableVertexAttribArray(pos_attr as GLuint);
            gl::VertexAttribPointer(
                pos_attr as GLuint,
                3,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                6 * mem::size_of::<GLfloat>() as i32,
                ptr::null(),
            );

            let c_color = CString::new("aColor").unwrap();
            let color_attr = gl::GetAttribLocation(program, c_color.as_ptr());
            gl::EnableVertexAttribArray(color_attr as GLuint);
            gl::VertexAttribPointer(
                color_attr as GLuint,
                3,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                6 * mem::size_of::<GLfloat>() as i32,
                (3 * mem::size_of::<GLfloat>()) as *const gl::types::GLvoid,
            );
        }
        Triangle { program, vao, vbo }
    }

    pub fn draw(&self) {
        unsafe {
            // Use the VAO created previously
            gl::BindVertexArray(self.vao);
            // Use shader program
            gl::UseProgram(self.program);
            // Draw a triangle from the 3 vertices
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            // Unbind the VAO
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Triangle {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
