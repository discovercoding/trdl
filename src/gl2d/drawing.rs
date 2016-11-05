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

const MAX_DEPTH : f32 = 5e5f32;

pub trait Window {
    fn set_context(&self);
    fn load_fn(&self, addr: &str) -> *const c_void;
}

pub struct Path {
    vertices: Vec<(f32, f32)>,
    control_point_1s: Vec<Option<(f32, f32)>>,
    control_point_2s: Vec<Option<(f32, f32)>>,
    fill_color: Option<[f32; 3]>,
    stroke: Option<([f32; 3], u32)>,
    is_closed: bool
}

impl Path {
    pub fn new(start: (f32, f32)) -> Self {
        let mut path = Path { vertices: Vec::new(), control_point_1s: Vec::new(),
            control_point_2s: Vec::new(), fill_color: None, stroke: None, is_closed: false };
        path.vertices.push(start);
        path
    }

    pub fn with_num_vertices(start: (f32, f32), num_vertices: usize) -> Self {
        let mut path = Path { vertices: Vec::with_capacity(num_vertices),
            control_point_1s: Vec::with_capacity(num_vertices),
            control_point_2s: Vec::with_capacity(num_vertices),
            fill_color: None, stroke: None, is_closed: false };
        path.vertices.push(start);
        path
    }

    pub fn curve_to(mut self, control_point_1: (f32, f32), control_point_2: (f32, f32),
                    end_point: (f32, f32),) -> Self {
        self.control_point_1s.push(Some(control_point_1));
        self.control_point_2s.push(Some(control_point_2));
        self.vertices.push(end_point);
        self
    }

    pub fn line_to(mut self, end_point: (f32, f32)) -> Self {
        self.control_point_1s.push(None);
        self.control_point_2s.push(None);
        self.vertices.push(end_point);
        self
    }

    pub fn close_path(mut self) -> Self {
        self.is_closed = true;
        if self.vertices[0] == self.vertices[self.vertices.len()-1] {
            self.vertices.pop();
        } else {
            self.control_point_1s.push(None);
            self.control_point_2s.push(None);
        }
        self
    }

    pub fn set_fill_color(mut self, red: f32, green: f32, blue: f32) -> Self {
        self.fill_color = Some([red as GLfloat, green as GLfloat, blue as GLfloat]);
        self
    }

    pub fn clear_fill_color(mut self) -> Self {
        self.fill_color = None;
        self
    }

    pub fn set_stroke(mut self, red: f32, green: f32, blue: f32, thickness: u32) -> Self {
        self.stroke = Some(([red as GLfloat, green as GLfloat, blue as GLfloat], thickness));
        self
    }

    pub fn clear_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }
}

pub struct Drawing<'a, W: Window + 'a> {
    window: &'a W,
    window_size: [GLfloat; 2],

    vertices: Vec<GLfloat>,
    control_point_1s: Vec<GLfloat>,
    control_point_2s: Vec<GLfloat>,
    fill_colors: Vec<GLfloat>,
    stroke_edges: Vec<GLfloat>,
    stroke_colors: Vec<GLfloat>,
    do_fill: Vec<GLint>,

    in_position: GLint,
    in_control_1: GLint,
    in_control_2: GLint,
    in_color: GLint,
    in_edge: GLint,
    in_stroke_color: GLint,
    in_do_fill: GLint,

    position_vbo: GLuint,
    control_1_vbo: GLuint,
    control_2_vbo: GLuint,
    color_vbo: GLuint,
    edge_vbo: GLuint,
    stroke_color_vbo: GLuint,
    do_fill_vbo: GLuint,

    shader_program: shader::ShaderProgram,
    vao_handle: GLuint,

    outer_tess_uniform: GLint,
    inner_tess_uniform: GLint,
    projection_uniform: GLint,
    window_size_uniform: GLint,

    ortho_proj: [GLfloat; 16],

    background_color: [GLfloat; 3],

    depth_idx: usize,
    num_tris: usize,
    remake: bool
}

impl<'a, W: Window> Drawing<'a, W> {
    pub fn new(window: &'a W, width: u32, height: u32, bg_red: f32, bg_green: f32, bg_blue: f32) ->
            Result<Drawing<W>, TrdlError> {
        window.set_context();
        gl::load_with(|symbol| window.load_fn(symbol));

        let vertex_shader_code = try!(read_file("shaders/vertex_shader.glsl"));
        let tess_control_shader_code = try!(read_file("shaders/tess_control_shader.glsl"));
        let tess_evaluation_shader_code = try!(read_file("shaders/tess_evaluation_shader.glsl"));
        let geometry_shader_code = try!(read_file("shaders/geometry_shader.glsl"));
        let fragment_shader_code = try!(read_file("shaders/fragment_shader.glsl"));
        let program;
        {
            let mut builder = shader::ShaderProgramBuilder::new();
            builder.set_vertex_shader(&vertex_shader_code);
            builder.set_tess_control_shader(&tess_control_shader_code);
            builder.set_tess_evaluation_shader(&tess_evaluation_shader_code);
            builder.set_geometry_shader(&geometry_shader_code);
            builder.set_fragment_shader(&fragment_shader_code);
            program = try!(builder.build_shader_program());
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
            let c_str = CString::new("in_stroke_color").unwrap();
            let in_stroke_color = gl::GetAttribLocation(program_id, c_str.as_ptr());
            let c_str = CString::new("in_do_fill").unwrap();
            let in_do_fill = gl::GetAttribLocation(program_id, c_str.as_ptr());

            let vao_handle = 0 as GLuint;

            // Create the buffer objects
            const NUM_VBO: i32 = 7;
            let mut vbo_handles = [0 as GLuint, 0 as GLuint, 0 as GLuint, 0 as GLuint,
                               0 as GLuint, 0 as GLuint, 0 as GLuint];
            gl::GenBuffers(NUM_VBO, mem::transmute(&vbo_handles[0]));

            let position_vbo = vbo_handles[0];
            let control_1_vbo = vbo_handles[1];
            let control_2_vbo = vbo_handles[2];
            let color_vbo = vbo_handles[3];
            let edge_vbo = vbo_handles[4];
            let stroke_color_vbo = vbo_handles[5];
            let do_fill_vbo = vbo_handles[6];

            Ok(Drawing {
                window: window,
                window_size: [gl!(width), gl!(height)],

                vertices: Vec::new(),
                control_point_1s: Vec::new(),
                control_point_2s: Vec::new(),
                fill_colors: Vec::new(),
                stroke_colors: Vec::new(),
                stroke_edges: Vec::new(),
                do_fill: Vec::new(),

                in_position: in_position,
                in_control_1: in_control_1,
                in_control_2: in_control_2,
                in_color: in_color,
                in_edge: in_edge,
                in_stroke_color: in_stroke_color,
                in_do_fill: in_do_fill,

                position_vbo: position_vbo,
                control_1_vbo: control_1_vbo,
                control_2_vbo: control_2_vbo,
                color_vbo: color_vbo,
                edge_vbo: edge_vbo,
                stroke_color_vbo: stroke_color_vbo,
                do_fill_vbo: do_fill_vbo,

                shader_program: program,
                vao_handle: vao_handle,

                outer_tess_uniform: -1,
                inner_tess_uniform: -1,
                projection_uniform: -1,
                window_size_uniform: -1,

                ortho_proj: Self::ortho(width, height),

                background_color: [gl!(bg_red), gl!(bg_green), gl!(bg_blue)],

                depth_idx: 0,
                num_tris: 0,
                remake: true
            })
        }
    }

    pub fn add_path(&mut self, path: Path) -> Result<(), TrdlError> {
        self.remake = true;
        if path.is_closed {
            self.add_closed_path(path)
        } else {
            self.add_open_path(path)
        }
    }

    fn add_closed_path(&mut self, path: Path) -> Result<(), TrdlError> {
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

        self.num_tris = indices.len() / 3;

        self.vertices.reserve(9 * self.num_tris);
        self.control_point_1s.reserve(6 * self.num_tris);
        self.control_point_2s.reserve(6 * self.num_tris);
        self.fill_colors.reserve(9 * self.num_tris);
        self.stroke_colors.reserve(9 * self.num_tris);
        self.stroke_edges.reserve(3 * self.num_tris);
        self.do_fill.reserve(3 * self.num_tris);

        let num_verts = path.vertices.len();
        self.depth_idx += 1;
        let depth = (self.depth_idx as f32) / MAX_DEPTH;

        for t in 0..self.num_tris {
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
            if let Some(stroke) = path.stroke {
                push3(&mut self.stroke_colors, stroke.0);
                let thickness = gl!(stroke.1);
                let (e0, e1, e2) = triangle_edges(indices[ti0], indices[ti1], indices[ti2], num_verts-1);
                self.stroke_edges.push(if e0 {thickness} else {ZERO});
                self.stroke_edges.push(if e1 {thickness} else {ZERO});
                self.stroke_edges.push(if e2 {thickness} else {ZERO});
            } else {
                push3(&mut self.stroke_colors, [ZERO, ZERO, ZERO]);
                self.stroke_edges.push(ZERO);
                self.stroke_edges.push(ZERO);
                self.stroke_edges.push(ZERO);
            }
            if let Some(fill_color) = path.fill_color {
                push3(&mut self.fill_colors, fill_color);
                self.do_fill.push(1 as GLint);
                self.do_fill.push(1 as GLint);
                self.do_fill.push(1 as GLint);

            } else {
                push3(&mut self.fill_colors, [ZERO, ZERO, ZERO]);
                self.do_fill.push(0 as GLint);
                self.do_fill.push(0 as GLint);
                self.do_fill.push(0 as GLint);
            }
        }
        Ok(())
    }

    fn make_extra_point(p0: (f32, f32), p1: (f32, f32)) -> Result<(f32, f32), TrdlError> {
        let offset = 5f32;
        if p1.0 > p0.0 {
            // x1 > x0
            if p1.1 > p0.1 {
                // y1 > y0
                Ok(((p0.0 + p1.0) / 2f32, p1.1))
            } else if p1.1 < p0.1 {
                // y1 < y0
                Ok(((p0.0 + p1.0) / 2f32, p0.1))
            } else {
                // y1 == y0
                Ok(((p0.0 + p1.0) / 2f32, p0.1 + offset))
            }
        } else if p1.0 < p0.0 {
            // x1 < x0
            if p1.1 > p0.1 {
                // y1 > y0
                Ok(((p0.0 + p1.0) / 2f32, p0.1))
            } else if p1.1 < p0.1 {
                // y1 < y0
                Ok(((p0.0 + p1.0) / 2f32, p1.1))
            } else {
                // y1 == y0
                Ok(((p0.0 + p1.0) / 2f32, p0.1 - offset))
            }
        } else {
            // x1 == x0
            if p1.1 > p0.1 {
                // y1 > y0
                Ok((p0.0 - offset, (p0.1 + p1.1) / 2f32))
            } else if p1.1 < p0.1 {
                // y1 < y0
                Ok((p0.0 + offset, (p0.1 + p1.1) / 2f32))
            } else {
                // y1 == y0
                Err(TrdlError::NonSimplePolygon)
            }
        }
    }

    fn add_open_path(&mut self, path: Path) -> Result<(), TrdlError> {

        if path.stroke == None {
            return Err(TrdlError::NoVisibleGeometry);
        }

        self.num_tris = path.vertices.len() - 1;

        self.vertices.reserve(9 * self.num_tris);
        self.control_point_1s.reserve(6 * self.num_tris);
        self.control_point_2s.reserve(6 * self.num_tris);
        self.fill_colors.append(&mut vec![gl!(0); 9 * self.num_tris]);
        self.stroke_colors.reserve(9 * self.num_tris);
        self.stroke_edges.reserve(3 * self.num_tris);
        self.do_fill.append(&mut vec![0 as GLint; 3 * self.num_tris]);

        self.depth_idx += 1;
        let depth = (self.depth_idx as f32) / MAX_DEPTH;

        for i in 0..self.num_tris {
            let v0 = path.vertices[i];
            let v1 = path.vertices[i + 1];
            let v2 = try!(Self::make_extra_point(v0, v1));
            self.vertices.push(v0.0); self.vertices.push(v0.1); self.vertices.push(depth);
            self.vertices.push(v1.0); self.vertices.push(v1.1); self.vertices.push(depth);
            self.vertices.push(v2.0); self.vertices.push(v2.1); self.vertices.push(depth);

            if let Some(cp1) = path.control_point_1s[i] {
                self.control_point_1s.push(cp1.0); self.control_point_1s.push(cp1.1);
                if let Some(cp2) = path.control_point_2s[i] {
                    self.control_point_2s.push(cp2.0); self.control_point_2s.push(cp2.1);
                } else {
                    panic!("Inconsistent control points");
                }
            } else {
                let (cp1, cp2) = bezier_line_control_points(v0, v1);
                self.control_point_1s.push(cp1.0); self.control_point_1s.push(cp1.1);
                self.control_point_2s.push(cp2.0); self.control_point_2s.push(cp2.1);
            }

            let (cp1, cp2) = bezier_line_control_points(v1, v2);
            self.control_point_1s.push(cp1.0); self.control_point_1s.push(cp1.1);
            self.control_point_2s.push(cp2.0); self.control_point_2s.push(cp2.1);

            let (cp1, cp2) = bezier_line_control_points(v2, v0);
            self.control_point_1s.push(cp1.0); self.control_point_1s.push(cp1.1);
            self.control_point_2s.push(cp2.0); self.control_point_2s.push(cp2.1);

            if let Some((stroke_color, stroke_thickness)) = path.stroke {
                push3(&mut self.stroke_colors, stroke_color);
                self.stroke_edges.push(gl!(0));
                self.stroke_edges.push(gl!(0));
                self.stroke_edges.push(gl!(stroke_thickness));
            } else {
                unreachable!()
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
                    (self.stroke_edges.len() * mem::size_of::<GLfloat> ()) as GLsizeiptr,
                    mem::transmute(&self.stroke_edges[0]),
                    gl::STATIC_DRAW);

                // populate the stroke color buffer
                gl::BindBuffer(gl::ARRAY_BUFFER, self.stroke_color_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                              (self.stroke_colors.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                               mem::transmute(&self.stroke_colors[0]),
                               gl::STATIC_DRAW);

                // populate the do fill buffer
                gl::BindBuffer(gl::ARRAY_BUFFER, self.do_fill_vbo);
                gl::BufferData(gl::ARRAY_BUFFER,
                               (self.do_fill.len() * mem::size_of::<GLint>()) as GLsizeiptr,
                               mem::transmute(&self.do_fill[0]),
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
                gl::EnableVertexAttribArray(5 as GLuint); // stroke color
                gl::EnableVertexAttribArray(6 as GLuint); // do fill

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
                gl::VertexAttribPointer(self.in_edge as GLuint, 1, gl::FLOAT,
                                        gl::FALSE as GLboolean, 0, ptr::null());
                gl::BindBuffer(gl::ARRAY_BUFFER, self.stroke_color_vbo);
                gl::VertexAttribPointer(self.in_stroke_color as GLuint, 3, gl::FLOAT,
                                        gl::FALSE as GLboolean, 0, ptr::null());
                gl::BindBuffer(gl::ARRAY_BUFFER, self.do_fill_vbo);
                gl::VertexAttribPointer(self.in_do_fill as GLuint, 1, gl::INT,
                                        gl::FALSE as GLboolean, 0, ptr::null());

                let program_id = self.shader_program.get_program_id();
                let c_str = CString::new("outer_tess".as_bytes()).unwrap();
                self.outer_tess_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
                let c_str = CString::new("inner_tess".as_bytes()).unwrap();
                self.inner_tess_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
                let c_str = CString::new("projection".as_bytes()).unwrap();
                self.projection_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
                let c_str = CString::new("window_size".as_bytes()).unwrap();
                self.window_size_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());

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

                if self.window_size_uniform >= 0 {
                    gl::Uniform2fv(self.window_size_uniform, 1,
                                  mem::transmute(&self.window_size[0]));
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
            TWO / gl!(width),  ZERO,              ZERO, ZERO,
            ZERO,              TWO / gl!(height), ZERO, ZERO,
            ZERO,              ZERO,              ONE,  ZERO,
            -ONE,             -ONE,               ZERO, ONE
        ]
    }

    pub fn set_size(&mut self, width: u32, height: u32) {
        self.ortho_proj = Self::ortho(width, height);
        self.remake = true;
        self.window_size = [gl!(width), gl!(height)];
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
            gl::DeleteBuffers(1, &self.stroke_color_vbo);
            gl::DeleteBuffers(1, &self.do_fill_vbo);
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
    let e1 = i0 == 0 && i2 == max || (i0 > i2 && i0 - i2 == 1);
    (e0, e1, e2)
}
