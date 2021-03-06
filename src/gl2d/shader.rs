use gl;
use gl::types::*;
use std::ptr;
use std::str;
use std::ffi::CString;
use super::super::TrdlError;

// Default pass through shader if none specified.
static DEFAULT_VERTEX_SHADER: &'static str =
    r"#version 400
    in vec3 vertex_position;
    in vec3 vertex_color;
    out vec3 color;
    void main() {
        color = vertex_color;
        gl_Position = vec4(vertex_position, 1.0);
    }";

// Default shader if none specified.
static DEFAULT_FRAGMENT_SHADER: &'static str =
    r"#version 400
    in vec3 color;
    layout(location = 0) out vec4 frag_color;
    void main() {
        frag_color = vec4(color, 1.0);
    }";

/// Allows user to build a shader program by adding individual shaders.
pub struct ShaderProgramBuilder<'a> {
    vertex_shader_code: &'a str,
    tess_control_shader_code: Option<&'a str>,
    tess_evaluation_shader_code: Option<&'a str>,
    geometry_shader_code: Option<&'a str>,
    fragment_shader_code: &'a str
}

/// Compiled shader program handles. Also cleans up on drop.
#[derive(Debug, PartialEq)]
pub struct ShaderProgram {
    vertex_shader_id: GLuint,
    tess_control_shader_id: Option<GLuint>,
    tess_evaluation_shader_id: Option<GLuint>,
    geometry_shader_id: Option<GLuint>,
    fragment_shader_id: GLuint,
    program_id: GLuint
}

impl<'a> ShaderProgramBuilder<'a> {
    /// Contrutor, default vertex and fragment shader, no other shaders.
    pub fn new() -> ShaderProgramBuilder<'a> {
        ShaderProgramBuilder {
            vertex_shader_code: DEFAULT_VERTEX_SHADER,
            tess_control_shader_code: None,
            tess_evaluation_shader_code: None,
            geometry_shader_code: None,
            fragment_shader_code: DEFAULT_FRAGMENT_SHADER
        }
    }

    /// Set the vertex shader code string
    pub fn set_vertex_shader<'b>(&'b mut self, code: &'a str) -> &'b mut Self {
        self.vertex_shader_code = code;
        self
    }

    /// Set the tessellation control shader code string
    pub fn set_tess_control_shader<'b>(&'b mut self, code: &'a str) -> &'b mut Self {
        self.tess_control_shader_code = Some(code);
        self
    } 

    /// Set the tessellation evaluation shader code string
    pub fn set_tess_evaluation_shader<'b>(&'b mut self, code: &'a str) -> &'b mut Self {
        self.tess_evaluation_shader_code = Some(code);
        self
    } 

    /// Set the geometry shader code string
    pub fn set_geometry_shader<'b>(&'b mut self, code: &'a str) -> &'b mut Self {
        self.geometry_shader_code = Some(code);
        self
    } 

    /// Set the fragment shader code string
    pub fn set_fragment_shader<'b>(&'b mut self, code: &'a str) -> &'b mut Self {
        self.fragment_shader_code = code;
        self
    }

    /// Compile all the shaders and link into a shader program.
    pub fn build_shader_program(&'a mut self) -> Result<ShaderProgram, TrdlError> {
        let vertex_shader_id = try!(self.compile_shader(self.vertex_shader_code, gl::VERTEX_SHADER));
        let tess_control_shader_id = match self.tess_control_shader_code {
            Some(code) => Some(try!(self.compile_shader(code, gl::TESS_CONTROL_SHADER))),
            None => None
        };
        let tess_evaluation_shader_id = match self.tess_evaluation_shader_code {
            Some(code) => Some(try!(self.compile_shader(code, gl::TESS_EVALUATION_SHADER))),
            None => None
        };
        let geometry_shader_id = match self.geometry_shader_code {
            Some(code) => Some(try!(self.compile_shader(code, gl::GEOMETRY_SHADER))),
            None => None
        };
        let fragment_shader_id = try!(self.compile_shader(self.fragment_shader_code, gl::FRAGMENT_SHADER));

        let program_id = try!(self.link_shaders(vertex_shader_id, tess_control_shader_id,
                              tess_evaluation_shader_id, geometry_shader_id, fragment_shader_id));
        Ok(ShaderProgram  { 
            vertex_shader_id: vertex_shader_id,
            tess_control_shader_id: tess_control_shader_id,
            tess_evaluation_shader_id: tess_evaluation_shader_id,
            geometry_shader_id: geometry_shader_id,
            fragment_shader_id: fragment_shader_id,
            program_id: program_id })
    }

    // compile a particular shader
    fn compile_shader(&'a self, code: &str, shader_type: GLuint) -> Result<GLuint, TrdlError> {
        unsafe {
            let shader_id = gl::CreateShader(shader_type);
            let c_str = try!(CString::new(code.as_bytes()).map_err(|_| TrdlError::NullString));
            gl::ShaderSource(shader_id, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader_id);

            let mut status = gl::FALSE as GLint;
            gl::GetShaderiv(shader_id, gl::COMPILE_STATUS, &mut status);
            if status == gl::FALSE as GLint {
                let mut length = 0 as GLint;
                gl::GetShaderiv(shader_id, gl::INFO_LOG_LENGTH, &mut length);
                println!("message length is {}", length);
                let mut message = vec![0u8; length as usize];
                gl::GetShaderInfoLog(shader_id, length, ptr::null_mut(), message.as_mut_ptr() as *mut GLchar);
                let err = match String::from_utf8(message) {
                    Ok(text) => {
                        println!("err: '{}'", text);
                        TrdlError::CompileError(text)
                    },
                    Err(_) => TrdlError::InvalidCompileError
                };
                Err(err)
            } else {
                Ok(shader_id)
            }
        }
    }

    fn link_shaders(&'a self, vertex_shader_id: GLuint, 
                        tess_control_shader_id: Option<GLuint>,
                        tess_evaluation_shader_id: Option<GLuint>,
                        geometry_shader_id: Option<GLuint>,
                        fragment_shader_id: GLuint) -> Result<GLuint, TrdlError> {
        unsafe {
            let program_id = gl::CreateProgram();
        
            gl::AttachShader(program_id, vertex_shader_id);
            if let Some(id) = tess_control_shader_id {
                gl::AttachShader(program_id, id);
            }
            if let Some(id) = tess_evaluation_shader_id {
                gl::AttachShader(program_id, id);
            }
            if let Some(id) = geometry_shader_id {
                gl::AttachShader(program_id, id);
            }
            gl::AttachShader(program_id, fragment_shader_id);

            gl::LinkProgram(program_id);

            let mut status = gl::FALSE as GLint;
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut status);
            if status == gl::FALSE as GLint {
                let mut length = 0 as GLint;
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut length);
                let mut message = Vec::with_capacity(length as usize);
                gl::GetProgramInfoLog(program_id, length, ptr::null_mut(), message.as_mut_ptr() as *mut GLchar);
                let err = match String::from_utf8(message) {
                    Ok(text) => TrdlError::CompileError(text),
                    Err(_) => TrdlError::InvalidCompileError
                };
                return Err(err);
            }

            gl::DetachShader(program_id, vertex_shader_id);
            if let Some(id) = tess_control_shader_id {
                gl::DetachShader(program_id, id);
            }
            if let Some(id) = tess_evaluation_shader_id {
                gl::DetachShader(program_id, id);
            }
            if let Some(id) = geometry_shader_id {
                gl::DetachShader(program_id, id);
            }
            gl::DetachShader(program_id, fragment_shader_id);

            Ok(program_id)
        }
    }
}

impl ShaderProgram {
    /// Get a numeric OpenGL handle to the shader program.
    pub fn get_program_id(&self) -> GLuint { self.program_id }
}

impl Drop for ShaderProgram {
    /// Delete the shader program and all the shaders.
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program_id);
            gl::DeleteShader(self.vertex_shader_id);
            if let Some(id) = self.tess_control_shader_id {
                gl::DeleteShader(id);
            }
            if let Some(id) = self.tess_evaluation_shader_id {
                gl::DeleteShader(id);
            }
            if let Some(id) = self.geometry_shader_id {
                gl::DeleteShader(id);
            }
            gl::DeleteShader(self.fragment_shader_id);
        }
    }
}

#[cfg(test)]
mod tests {

    use std::io::prelude::*;
    use std::fs::File;
    use std::io;
    use glutin;
    use gl;

    use super::ShaderProgramBuilder;
    use super::ShaderProgram;
    use super::ShaderError;

    fn read_file(file_name: &str) -> io::Result<String> {
        let mut contents = String::new();
        let mut f = try!(File::open(file_name));
        try!(f.read_to_string(&mut contents));
        Ok(contents)
    }

    #[test]
    fn compile_defaults() {

        let window = glutin::Window::new().unwrap();
        unsafe { window.make_current() };
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

        let mut builder = ShaderProgramBuilder::new();
        match builder.build_shader_program() {
            Ok(_) => { },
            Err(_) => assert!(false)
        };
    }
}