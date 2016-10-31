extern crate gl;

use std::mem;
use std::ffi::CString;
use std::ptr;
use std::io::prelude::*;
use std::fs::File;
use std::collections::hash_map::HashMap;
use std::os::raw::c_void;
use gl::types::*;
use super::shader;
use super::super::triangulation::triangulate;
use super::super::TrdlError;

macro_rules! gl {
    ($e:expr) => ($e as GLfloat);
}
// this just saves us from writing "gl!()" in common cases
const ZERO:  GLfloat = gl!(0);
const ONE:   GLfloat = gl!(1);
const TWO:   GLfloat = gl!(2);
const THREE: GLfloat = gl!(3);

pub trait Window {
    fn set_context(&self);
    fn load_fn(&self, addr: &str) -> *const c_void;
}

pub struct FilledPath {
    vertices: Vec<(f32, f32)>,
    control_point_1s: Vec<Option<(f32, f32)>>,
    control_point_2s: Vec<Option<(f32, f32)>>,
    fill_color: [f32; 3],
    stroke: Option<([f32; 3], u32)>
}

impl FilledPath {
    pub fn new() -> Self {
        FilledPath { vertices: Vec::new(), control_point_1s: Vec::new(),
            control_point_2s: Vec::new(), fill_color: [1f32, 1f32, 1f32], stroke: None }
    }

    pub fn with_num_vertices(num_vertices: usize) -> Self {
        FilledPath { vertices: Vec::with_capacity(num_vertices),
            control_point_1s: Vec::with_capacity(num_vertices),
            control_point_2s: Vec::with_capacity(num_vertices),
            fill_color: [1f32, 1f32, 1f32], stroke: None }
    }

    pub fn add_bezier_curve(mut self, start_point: (f32, f32), control_point_1: (f32, f32), control_point_2: (f32, f32)) -> Self {
        self.vertices.push(start_point);
        self.control_point_1s.push(Some(control_point_1));
        self.control_point_2s.push(Some(control_point_2));
        self
    }

    pub fn add_straight_line(mut self, start_point: (f32, f32)) -> Self {
        self.vertices.push(start_point);
        self.control_point_1s.push(None);
        self.control_point_2s.push(None);
        self
    }

    pub fn set_fill_color(mut self, red: f32, green: f32, blue: f32) -> Self {
        self.fill_color = [red as GLfloat, green as GLfloat, blue as GLfloat];
        self
    }

    pub fn set_stroke(mut self, red: f32, green: f32, blue: f32, thickness: u32) -> Self {
        self.stroke = Some(([red as GLfloat, green as GLfloat, blue as GLfloat],
                                 thickness));
        self
    }

    pub fn clear_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }
}

pub struct Drawing<'a, W: Window + 'a> {
    window: &'a W,
    vertices: Vec<GLfloat>,
    control_point_1s: Vec<GLfloat>,
    control_point_2s: Vec<GLfloat>,
    fill_colors: Vec<GLfloat>,
    stroke_colors: Vec<GLfloat>,
    stroke_edges: Vec<GLuint>,

    shader_program: shader::ShaderProgram,

    in_position: GLint,
    in_control_1: GLint,
    in_control_2: GLint,
    in_color: GLint,
    in_edge: GLint,

    vao_handle: GLuint,

    position_vbo: GLuint,
    control_1_vbo: GLuint,
    control_2_vbo: GLuint,
    color_vbo: GLuint,
    edge_vbo: GLuint,

    outer_tess_uniform: GLint,
    inner_tess_uniform: GLint,
    projection_uniform: GLint,

    ortho_proj: [GLfloat; 16],

    background_color: [GLfloat; 3],

    remake: bool
}

impl<'a, W: Window> Drawing<'a, W> {
    pub fn new(window: &'a W, width: u32, height: u32) -> Drawing<W> {
        window.set_context();
        gl::load_with(|symbol| window.load_fn(symbol));

        let vertex_shader_code = read_file("vertex_shader.glsl").unwrap();
        let tess_control_shader_code = read_file("tess_control_shader.glsl").unwrap();
        let tess_evaluation_shader_code = read_file("tess_evaluation_shader.glsl").unwrap();
        let geometry_shader_code = read_file("geometry_shader.glsl").unwrap();
        let fragment_shader_code = read_file("fragment_shader.glsl").unwrap();
        let program;
        {
            let mut builder = shader::ShaderProgramBuilder::new();
            builder.set_vertex_shader(&vertex_shader_code);
            builder.set_tess_control_shader(&tess_control_shader_code);
            builder.set_tess_evaluation_shader(&tess_evaluation_shader_code);
            builder.set_geometry_shader(&geometry_shader_code);
            builder.set_fragment_shader(&fragment_shader_code);
            program = builder.build_shader_program().unwrap();
        }
        let program_id = program.get_program_id();
        unsafe {
            let c_str = CString::new("in_position").unwrap();
            let in_position = gl::GetAttribLocation(program_id, c_str.as_ptr());
            let c_str = CString::new("in_control_1").unwrap();
            let in_control_1 = gl::GetAttribLocation(program_id, c_str.as_ptr());
            let c_str = CString::new("in_control_2").unwrap();
            let in_control_2 = gl::GetAttribLocation(program_id, c_str.as_ptr());
            let c_str = CString::new("in_color").unwrap();
            let in_color = gl::GetAttribLocation(program_id, c_str.as_ptr());
            let c_str = CString::new("in_edge").unwrap();
            let in_edge = gl::GetAttribLocation(program_id, c_str.as_ptr());

            let vao_handle = 0 as GLuint;

            // Create the buffer objects
            const NUM_VBO: i32 = 5;
            let vbo_handles = [0 as GLuint, 0 as GLuint, 0 as GLuint, 0 as GLuint, 0 as GLuint];
            gl::GenBuffers(NUM_VBO, mem::transmute(&vbo_handles[0]));

            let position_vbo = vbo_handles[0];
            let control_1_vbo = vbo_handles[1];
            let control_2_vbo = vbo_handles[2];
            let color_vbo = vbo_handles[3];
            let edge_vbo = vbo_handles[4];

            Drawing {
                window: window,
                vertices: Vec::new(),
                control_point_1s: Vec::new(),
                control_point_2s: Vec::new(),
                fill_colors: Vec::new(),
                stroke_colors: Vec::new(),
                stroke_edges: Vec::new(),

                shader_program: program,

                in_position: in_position,
                in_control_1: in_control_1,
                in_control_2: in_control_2,
                in_color: in_color,
                in_edge: in_edge,

                vao_handle: vao_handle,

                position_vbo: position_vbo,
                control_1_vbo: control_1_vbo,
                control_2_vbo: control_2_vbo,
                color_vbo: color_vbo,
                edge_vbo: edge_vbo,

                outer_tess_uniform: -1,
                inner_tess_uniform: -1,
                projection_uniform: -1,

                ortho_proj: Self::ortho(width, height),

                background_color: [gl!(0.2), gl!(0.2), gl!(0.2)],

                remake: true
            }
        }
    }

    pub fn add_filled_path(&mut self, path: FilledPath, depth: f32) ->Result<(), TrdlError> {
        self.remake = true;
        let mut control_point_map = HashMap::new();
        let last = path.vertices.len() - 1;
        for i in 0..last {
            if let Some(cp1) = path.control_point_1s[i] {
                if let Some(cp2) = path.control_point_2s[i] {
                    control_point_map.insert((i, i+1), (cp1, cp2));
                } else {
                    panic!("inconsistent control points!");
                }
            }
        }
        if let Some(cp1) = path.control_point_1s[last] {
            if let Some(cp2) = path.control_point_2s[last] {
                control_point_map.insert((last, 0), (cp1, cp2));
            } else {
                panic!("inconsistent control points!");
            }
        }

        let indices = try!(triangulate(&path.vertices));
        let num_tris = indices.len() / 3;

        self.vertices.reserve(3 * num_tris);
        self.control_point_1s.reserve(2 * num_tris);
        self.control_point_2s.reserve(2 * num_tris);
        self.fill_colors.reserve(3 * num_tris);
        self.stroke_colors.reserve(3 * num_tris);
        self.stroke_edges.reserve(num_tris);

        let num_verts = path.vertices.len();
        for t in 0..num_tris {
            let ti0 = 3*t;
            let ti1 = ti0+1;
            let ti2 = ti0+2;
            get_control_points(&path.vertices, indices[ti0], indices[ti1], depth,
                               &mut control_point_map, &mut self.vertices,
                               &mut self.control_point_1s, &mut self.control_point_2s);
            get_control_points(&path.vertices, indices[ti1], indices[ti2], depth,
                               &mut control_point_map, &mut self.vertices,
                               &mut self.control_point_1s, &mut self.control_point_2s);
            get_control_points(&path.vertices, indices[ti2], indices[ti0], depth,
                               &mut control_point_map, &mut self.vertices,
                               &mut self.control_point_1s, &mut self.control_point_2s);
            push3(&mut self.fill_colors, path.fill_color);
            if let Some(stroke) = path.stroke {
                let thick = stroke.1;
                push3(&mut self.stroke_colors, stroke.0);
                let (e0, e1, e2) = triangle_edges(ti0, ti1, ti2, num_verts);
                self.stroke_edges.push(if e0 {thick} else {0u32});
                self.stroke_edges.push(if e1 {thick} else {0u32});
                self.stroke_edges.push(if e2 {thick} else {0u32});
            } else {
                push3(&mut self.stroke_colors, [0f32, 0f32, 0f32]);
                self.stroke_edges.push(0u32);
                self.stroke_edges.push(0u32);
                self.stroke_edges.push(0u32);
            }
        }
        Ok(())
    }

    pub fn make_current(&self) {
        self.window.set_context();
    }

    pub fn draw(&mut self) {
        unsafe {
            if self.remake {
                // Populate the position buffer
                gl::BindBuffer(gl::ARRAY_BUFFER, self.position_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                    (self.vertices.len() * mem::size_of::<GLfloat> ()) as GLsizeiptr,
                    mem::transmute(&self.vertices[0]),
                    gl::STATIC_DRAW);

                // Populate the control points buffers
                gl::BindBuffer(gl::ARRAY_BUFFER, self.control_1_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                    (self.control_point_1s.len() * mem::size_of::<GLfloat> ()) as GLsizeiptr,
                    mem::transmute(&self.control_point_1s[0]),
                    gl::STATIC_DRAW);

                // Populate the control points buffers
                gl::BindBuffer(gl::ARRAY_BUFFER, self.control_2_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                    (self.control_point_2s.len() * mem::size_of::<GLfloat> ()) as GLsizeiptr,
                    mem::transmute(&self.control_point_2s[0]),
                    gl::STATIC_DRAW);

                // Populate color buffer
                gl::BindBuffer(gl::ARRAY_BUFFER, self.color_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                    (self.fill_colors.len() * mem::size_of::<GLfloat> ()) as GLsizeiptr,
                    mem::transmute(&self.fill_colors[0]),
                    gl::STATIC_DRAW);

                // Populate the edge buffer
                gl::BindBuffer(gl::ARRAY_BUFFER, self.edge_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                    (self.stroke_edges.len() * mem::size_of::<GLuint> ()) as GLsizeiptr,
                    mem::transmute(&self.stroke_edges[0]),
                    gl::STATIC_DRAW);

                gl::PatchParameteri(gl::PATCH_VERTICES, 3);

                // Create and set-up the vertex array object
                gl::GenVertexArrays(1, &mut self.vao_handle);
                gl::BindVertexArray(self.vao_handle);

                // Enable the vertex attribute arrays
                gl::EnableVertexAttribArray(0 as GLuint); // position
                gl::EnableVertexAttribArray(1 as GLuint); // control point 1
                gl::EnableVertexAttribArray(2 as GLuint); // control point 2
                gl::EnableVertexAttribArray(3 as GLuint); // color
                gl::EnableVertexAttribArray(4 as GLuint); // edge

                gl::BindBuffer(gl::ARRAY_BUFFER, self.position_vbo);
                gl::VertexAttribPointer(self.in_position as GLuint, 3, gl::FLOAT,
                gl::FALSE as GLboolean, 0, ptr::null());
                gl::BindBuffer(gl::ARRAY_BUFFER, self.control_1_vbo);
                gl::VertexAttribPointer(self.in_control_1 as GLuint, 2, gl::FLOAT,
                gl::FALSE as GLboolean, 0, ptr::null());
                gl::BindBuffer(gl::ARRAY_BUFFER, self.control_2_vbo);
                gl::VertexAttribPointer(self.in_control_2 as GLuint, 2, gl::FLOAT,
                gl::FALSE as GLboolean, 0, ptr::null());
                gl::BindBuffer(gl::ARRAY_BUFFER, self.color_vbo);
                gl::VertexAttribPointer(self.in_color as GLuint, 3, gl::FLOAT,
                gl::FALSE as GLboolean, 0, ptr::null());
                gl::BindBuffer(gl::ARRAY_BUFFER, self.edge_vbo);
                gl::VertexAttribPointer(self.in_edge as GLuint, 1, gl::UNSIGNED_INT,
                gl::FALSE as GLboolean, 0, ptr::null());

                let program_id = self.shader_program.get_program_id();
                let c_str = CString::new("outer_tess".as_bytes()).unwrap();
                self.outer_tess_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
                let c_str = CString::new("inner_tess".as_bytes()).unwrap();
                self.inner_tess_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
                let c_str = CString::new("projection".as_bytes()).unwrap();
                self.projection_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());

                gl::UseProgram(self.shader_program.get_program_id());

                if self.outer_tess_uniform >= 0 {
                    gl::Uniform1i(self.outer_tess_uniform, 32);
                }

                if self.inner_tess_uniform >= 0 {
                    gl::Uniform1i(self.inner_tess_uniform, 1);
                }

                if self.projection_uniform >= 0 {
                    gl::UniformMatrix4fv(self.projection_uniform, 1, gl::FALSE as GLboolean,
                                         mem::transmute(&self.ortho_proj[0]));
                }

                gl::Enable(gl::DEPTH_TEST);

                gl::ClearColor(self.background_color[0], self.background_color[1], self.background_color[2], 1.0);

                self.remake = false;
            }

            // Clear the screen
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            gl::BindVertexArray(self.vao_handle);
            gl::DrawArrays(gl::PATCHES, 0, self.vertices.len() as GLint);
        }
    }
    fn ortho(width: u32, height: u32) -> [GLfloat; 16] {
        [
            TWO / gl!(width),  ZERO, ZERO, ZERO,
            ZERO, TWO / gl!(height), ZERO, ZERO,
            ZERO, ZERO, ONE, ZERO,
            -ONE, -ONE, ZERO, ONE
        ]
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.ortho_proj = Self::ortho(width, height);
        self.remake = true;
    }
}

impl<'a, W: Window> Drop for Drawing<'a, W> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.position_vbo);
            gl::DeleteBuffers(1, &self.control_1_vbo);
            gl::DeleteBuffers(1, &self.control_2_vbo);
            gl::DeleteBuffers(1, &self.color_vbo);
            gl::DeleteBuffers(1, &self.edge_vbo);
            gl::DeleteVertexArrays(1, &self.vao_handle);
        }
    }
}

fn read_file(file_name: &str) -> Result<String, TrdlError> {
    let mut contents = String::new();
    let mut f = try!(File::open(file_name));
    try!(f.read_to_string(&mut contents));
    Ok(contents)
}

fn bezier_line_control_points(first: (GLfloat, GLfloat), last: (GLfloat, GLfloat))-> 
        ((GLfloat, GLfloat), (GLfloat, GLfloat)) {
    let dx = (last.0 - first.0) / THREE;
    let dy = (last.1 - first.1) / THREE;

    let v1 = (first.0 + dx, first.1 + dy);
    (v1, (v1.0 + dx, v1.1 + dy))
}

// For straight lines, the control points are calculated from the end points,
// for curves they have to be specified, this function figures out if control
// points for a particular pair of end points have been previously specified
// or calculated and reuses them, or calculates them otherwise.
fn get_control_points(polygon: &Vec<(GLfloat, GLfloat)>, i0: usize, i1: usize, depth: GLfloat,
        control_point_map: &mut HashMap<(usize, usize), ((GLfloat, GLfloat), (GLfloat, GLfloat))>,
        vs: &mut Vec<GLfloat>, cp1s: &mut Vec<GLfloat>, cp2s: &mut Vec<GLfloat>) {
    let v0 = polygon[i0];
    let v1 = polygon[i1];
    vs.push(v0.0);
    vs.push(v0.1);
    vs.push(depth);
    let cp1;
    let cp2;
    let mut insert = false;
    if let Some(cp12) = control_point_map.get(&(i0, i1)) {
        cp1 = cp12.0;
        cp2 = cp12.1;
    } else {
        let (a, b) = bezier_line_control_points(v0, v1);
        cp1 = a;
        cp2 = b;
        insert = true;
    }
    if insert { control_point_map.insert((i0, i1), (cp1, cp2)); }
    
    cp1s.push(cp1.0);
    cp1s.push(cp1.1);
    cp2s.push(cp2.0);
    cp2s.push(cp2.1);
}

fn push3(vec: &mut Vec<GLfloat>, value: [f32; 3]) {
    vec.push(value[0]);
    vec.push(value[1]);
    vec.push(value[2]);

    vec.push(value[0]);
    vec.push(value[1]);
    vec.push(value[2]);

    vec.push(value[0]);
    vec.push(value[1]);
    vec.push(value[2]);
}

fn triangle_edges(i0: usize, i1: usize, i2: usize, max: usize) -> (bool, bool, bool) { 
    let e2 = i1 == 0 && i0 == max || (i1 > i0 && i1 - i0 == 1);
    let e0 = i2 == 0 && i1 == max || (i2 > i1 && i2 - i1 == 1);
    let e1 = i0 == 0 && i2 == max || (i2 > i2 && i0 - i2 == 1);
    (e0, e1, e2)
}
