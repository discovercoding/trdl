extern crate gl;

use std::mem;
use std::ffi::CString;
use std::ptr;
use std::io::prelude::*;
use std::fs::File;
use std::collections::hash_map::HashMap;
use std::os::raw::c_void;
use std::f32;
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
const TOL: f32 = 1e-5f32;

/// Users of the library must provide a window with these functions, they are provided by winit,
/// glutin, GLFW-rs
pub trait Window {
    fn set_context(&self);
    fn load_fn(&self, addr: &str) -> *const c_void;
}

/// All shapes in TRDL are paths, which are built by adding lines curves and arcs.
pub struct Path {
    vertices: Vec<(f32, f32)>,
    control_point_1s: Vec<Option<(f32, f32)>>,
    control_point_2s: Vec<Option<(f32, f32)>>,
    fill_color: Option<[f32; 3]>,
    stroke: Option<([f32; 3], u32)>,
    is_closed: bool
}

impl Path {
    /// Constructor, takes the first point in the path as input.
    pub fn new(start: (f32, f32)) -> Self {
        let mut path = Path { vertices: Vec::new(), control_point_1s: Vec::new(),
            control_point_2s: Vec::new(), fill_color: None, stroke: None, is_closed: false };
        path.vertices.push(start);
        path
    }

    /// Add a straight line segment from the current point to end_point, which becomes the current
    /// point.
    pub fn line_to(mut self, end_point: (f32, f32)) -> Self {
        self.control_point_1s.push(None);
        self.control_point_2s.push(None);
        self.vertices.push(end_point);
        self
    }

    /// Add a cubic Bezier curve starting at the current point to end_point, which becomes the
    /// current point. The curves defined by control_point_1 and control_point_2. The current point
    /// can be considered control_point_0, and the end_point control_point_3.
    pub fn curve_to(mut self, control_point_1: (f32, f32), control_point_2: (f32, f32),
                    end_point: (f32, f32),) -> Self {
        self.control_point_1s.push(Some(control_point_1));
        self.control_point_2s.push(Some(control_point_2));
        self.vertices.push(end_point);
        self
    }

    /// Add an elliptical arc starting at the current point to end_point, which becomes the current
    /// point. The arc is defined by x_radius and y_radius, angle, which describe the whole ellipse
    /// of which the arc is a part. It is also described by is_positive_sweep which determine if the
    /// arc curves clockwise or counter clockwise and is_large_arc which determines if the arc takes
    /// the short path or long path to the end point.
    /// See https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
    pub fn arc_to(mut self, x_radius: f32, y_radius: f32, angle: f32, end_point: (f32, f32),
              is_large_arc: bool, is_positive_sweep: bool) -> Self {
        if let Ok((center, start_angle, sweep_angle)) =
            self.get_ellipse_params(x_radius, y_radius, angle, end_point,
                                    is_large_arc, is_positive_sweep) {
            // approximate a circular arc (radius = x_radius) with Bezier splines

            // break it into quarter-circle arcs
            let mut num_arcs = (sweep_angle.abs() / f32::consts::FRAC_PI_2).floor() as usize;
            // and 1 less than 90 degree arc
             let remainder = sweep_angle.abs() - f32::consts::FRAC_PI_2 * (num_arcs as f32);
            let mut points = Vec::new();
            if num_arcs > 0 {
                points.append(&mut Self::quarter_circle(x_radius, num_arcs, sweep_angle >= TOL));
            }
            if remainder.abs() > TOL {
                points.append(&mut Self::less_than_quarter_circle(x_radius, remainder, num_arcs,
                                                                  sweep_angle >= TOL));
                num_arcs += 1;
            }
            // now make the circular arc start at the right place
            Self::rotate_points(&mut points, start_angle);
            // now make it into an ellipse
            let scale = y_radius / x_radius;
            for p in &mut points {
                *p = (p.0, p.1 * scale);
            }
            // rotate the ellipse
            Self::rotate_points(&mut points, angle);
            // center it in the correct location
            for p in &mut points {
                let x = p.0;
                let y = p.1;
                *p = (x + center.0, y + center.1);
            }
            // add the curves
            for i in 0..num_arcs {
                let k = (i * 3) as usize;
                self = self.curve_to(points[k], points[k + 1], points[k + 2]);
            }
        } else {
            self = self.line_to(end_point);
        }
        self
    }

    /// Makes a polygon closed so it can be filled with color. If the last point is not the same as
    /// the first point, they are connected with a straight line.
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

    /// Sets the fill color for closed shapes.
    pub fn set_fill_color(mut self, red: f32, green: f32, blue: f32) -> Self {
        self.fill_color = Some([red as GLfloat, green as GLfloat, blue as GLfloat]);
        self
    }

    /// Removes the fill color if previously set, shape will be drawn unfilled.
    pub fn clear_fill_color(mut self) -> Self {
        self.fill_color = None;
        self
    }

    /// Set the stroke color and thickness of closed or open paths.
    pub fn set_stroke(mut self, red: f32, green: f32, blue: f32, thickness: u32) -> Self {
        self.stroke = Some(([red as GLfloat, green as GLfloat, blue as GLfloat], thickness));
        self
    }

    /// Clears the stroke of a path, shape will be drawn unstroked.
    pub fn clear_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }

    /// Create a rectangle path.
    pub fn rectangle(center: (f32, f32), width: f32, height: f32, angle: f32) -> Self {
        let x2 = width/2f32;
        let x1 = -x2;
        let y2 = height/2f32;
        let y1 = -y2;

        let mut points = [(x1, y1), (x2, y1), (x2, y2), (x1, y2)];
        Self::rotate_points(&mut points, angle);
        for p in &mut points {
            *p = (p.0 + center.0, p.1 + center.1);
        }
        Self::new(points[0]).line_to(points[1]).line_to(points[2]).line_to(points[3]).close_path()
    }

    /// Create an ellipse path.
    pub fn ellipse(center: (f32, f32), x_radius: f32, y_radius:f32, angle: f32) -> Self {
        let mut points = [(x_radius, 0f32), (0f32, y_radius)];
        Self::rotate_points(&mut points, angle);
        for p in &mut points {
            *p = (p.0 + center.0, p.1 + center.1);
        }
        Self::new(points[0]).arc_to(x_radius, y_radius, angle, points[1], false, true).
            arc_to(x_radius, y_radius, angle, points[0], true, true).close_path()
    }

    // calculate the center point, start angle and sweep angle of the arc.
    fn get_ellipse_params(&mut self, x_radius: f32, y_radius: f32, angle: f32, end_point: (f32, f32),
                          is_large_arc: bool, is_positive_sweep: bool) ->
                          Result<((f32, f32), f32, f32), TrdlError> {
        // math taken from https://www.w3.org/TR/SVG/implnote.html#ArcImplementationNotes
        // up to the point where we get the center point.
        let start_point = self.vertices[self.vertices.len() - 1];
        let xt = (start_point.0 - end_point.0) / 2f32;
        let yt = (start_point.1 - end_point.1) / 2f32;
        let cos_phi = angle.cos();
        let sin_phi = angle.sin();
        let x = cos_phi * xt + sin_phi * yt;
        let y = -sin_phi * xt + cos_phi * yt;
        let x_sq = x * x;
        let y_sq = y * y;

        let (x_radius, y_radius) = try!(Self::fix_radii(x_radius, y_radius, x, y));

        let rx_sq = x_radius * x_radius;
        let ry_sq = y_radius * y_radius;
        let xt = x_radius * y / y_radius;
        let yt = -y_radius * x / x_radius;

        let mut radical = ((rx_sq*ry_sq - rx_sq*y_sq - ry_sq*x_sq) /
                       (rx_sq*y_sq + ry_sq*x_sq)).sqrt();
        if is_large_arc == is_positive_sweep {
            radical = -radical;
        }

        let cxt = radical * xt;
        let cyt = radical * yt;
        let xt = (start_point.0 + end_point.0) / 2f32;
        let yt = (start_point.1 + end_point.1) / 2f32;

        let cx = cos_phi*cxt - sin_phi*cyt + xt;
        let cy = sin_phi*cxt + cos_phi*cyt + yt;

        let xt = (x - cxt) / x_radius;
        let yt = (y - cyt) / y_radius;
        let xt2 = (-x - cxt) / x_radius;
        let yt2 = (-y - cyt) / y_radius;

        let start_angle = Self::get_angle(1f32, 0f32, xt, yt);
        let mut sweep_angle = Self::get_angle(xt, yt, xt2, yt2);

        if is_positive_sweep && sweep_angle < TOL {
            sweep_angle += 2f32*f32::consts::PI;
        } else if !is_positive_sweep && sweep_angle > -TOL {
            sweep_angle -= 2f32*f32::consts::PI;
        }
        Ok(((cx, cy), start_angle, sweep_angle))
    }

    // make sure the radii are big enough to make sense.
    fn fix_radii(x_radius: f32, y_radius: f32, x_sq: f32, y_sq: f32) -> Result<(f32, f32), TrdlError> {
        if x_radius < TOL || y_radius == TOL { return Err(TrdlError::ArcToIsLineTo); }
        let x_radius = x_radius.abs();
        let y_radius = y_radius.abs();
        let gamma = x_sq / (x_radius * x_radius) + y_sq / (y_radius * y_radius);
        if gamma < 1f32 {
            Ok((x_radius, y_radius))
        } else {
            let gamma = gamma.sqrt();
            Ok((x_radius * gamma, y_radius * gamma))
        }
    }

    // used to calulate the start angle and sweep angle.
    fn get_angle(ux: f32, uy: f32, vx: f32, vy: f32) -> f32 {
        let u_mag = (ux*ux + uy*uy).sqrt();
        let v_mag = (vx*vx + vy*vy).sqrt();
        let arg = (ux*vx + uy*vy) / (u_mag * v_mag);
        let angle = arg.acos();
        if ux*vy-uy*vx < -TOL {
            -angle
        } else {
            angle
        }
    }

    // Makes 1, 2, 3, or 4 quarter circle arcs
    fn quarter_circle(radius: f32, num_quadrants: usize,
                      is_positive_sweep: bool) -> Vec<(f32, f32)> {
        // math is from http://pomax.github.io/bezierinfo/#circles_cubic
        let magic =  radius * 0.5522847498308; // 4f32 * (2f32.sqrt() - 1f32) / 3f32;

        let mut result = Vec::with_capacity(4 * num_quadrants);
        if is_positive_sweep {
            result.append(&mut vec![(radius, magic), (magic, radius), (0f32, radius)]);
            if num_quadrants > 1 {
                result.append(&mut vec![(-magic, radius), (-radius, magic), (-radius, 0f32)]);
            }
            if num_quadrants > 2 {
                result.append(&mut vec![(-radius, -magic), (-magic, -radius), (0f32, -radius)]);
            }
            if num_quadrants > 3 {
                result.append(&mut vec![(magic, -radius), (radius, -magic), (0f32, -radius)]);
            }
        } else {
            result.append(&mut vec![(radius, -magic), (magic, -radius), (0f32, -radius)]);
            if num_quadrants > 1 {
                result.append(&mut vec![(-magic, -radius), (-radius, -magic), (-radius, 0f32)]);
            }
            if num_quadrants > 2 {
                result.append(&mut vec![(-radius, magic), (-magic, radius), (0f32, radius)]);
            }
            if num_quadrants > 3 {
                result.append(&mut vec![(magic, radius), (radius, magic), (0f32, radius)]);
            }
        }
        result
    }

    // makes a circular arc with less than 90 degrees
    fn less_than_quarter_circle(radius: f32, angle: f32, quadrant: usize,
                                is_positive_sweep: bool) -> Vec<(f32, f32)> {
        // math is from http://pomax.github.io/bezierinfo/#circles_cubic
        let s = angle.sin();
        let c = angle.cos();
        let f = 4f32 / 3f32 * (angle / 4f32).tan();
        let th1 = c + f*s;
        let th2 = s - f*c;

        if is_positive_sweep {
            if quadrant == 0 {
                vec![(radius, f * radius), (th1 * radius, th2 * radius), (c * radius, s * radius)]
            } else if quadrant == 1 {
                vec![(-f * radius, radius), (-th2 * radius, th1 * radius), (-s * radius, c * radius)]
            } else if quadrant == 2 {
                vec![(-radius, -f * radius), (-th1 * radius, -th2 * radius), (-c * radius, -s * radius)]
            } else {
                vec![(f * radius, -radius), (th2 * radius, -th1 * radius), (s * radius, -c * radius)]
            }
        } else {
            if quadrant == 0 {
                vec![(radius, -f * radius), (th1 * radius, -th2 * radius), (c * radius, -s * radius)]
            } else if quadrant == 1 {
                vec![(-f * radius, -radius), (-th2 * radius, -th1 * radius), (-s * radius, -c * radius)]
            } else if quadrant == 2 {
                vec![(-radius, f * radius), (-th1 * radius, th2 * radius), (-c * radius, s * radius)]
            } else {
                vec![(f * radius, radius), (th2 * radius, th1 * radius), (s * radius, c * radius)]
            }
        }
    }

    // rotates points by angle.
    fn rotate_points(points: &mut [(f32, f32)], angle: f32) {
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();
        for p in points {
            let x = p.0;
            let y = p.1;
            *p = (cos_angle*x - sin_angle*y, sin_angle*x + cos_angle*y);
        }
    }
}

/// Manages everything under the hood. Paths are added to the drawing and then drawn.
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
    /// Constructor, a window, window size and background color.
    pub fn new(window: &'a W, width: u32, height: u32, bg_red: f32, bg_green: f32, bg_blue: f32) ->
            Result<Drawing<W>, TrdlError> {
        window.set_context();
        gl::load_with(|symbol| window.load_fn(symbol));

        // load the shaders and compile them into a shader program
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

        // setup the inputs to the vertex shader
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
            let vbo_handles = [0 as GLuint, 0 as GLuint, 0 as GLuint, 0 as GLuint,
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

    /// Add a path to the drawing.
    pub fn add_path(&mut self, path: Path) -> Result<(), TrdlError> {
        self.remake = true;
        if path.is_closed {
            self.add_closed_path(path)
        } else {
            self.add_open_path(path)
        }
    }

    // Triangulate the path.
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
        let depth = (MAX_DEPTH - (self.depth_idx as f32)) / MAX_DEPTH;

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

    // make a new point such that the 3 points make a triangle, be careful that the order makes a
    // counter clockwise winding
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

    // take each segment between the points of the path and add a point to turn each one into an
    // unfilled triangle.
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
        let depth = (MAX_DEPTH - (self.depth_idx as f32)) / MAX_DEPTH;

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

    /// Make this drawings render context the current one for the window.
    pub fn make_current(&self) {
        self.window.set_context();
    }

    /// Clear all paths in a drawing so the drawing can be reused.
    pub fn clear_paths(&mut self) {
        self.vertices.clear();
        self.control_point_1s.clear();
        self.control_point_2s.clear();
        self.fill_colors.clear();
        self.stroke_colors.clear();
        self.stroke_edges.clear();
        self.do_fill.clear();
        self.depth_idx = 0;
        self.num_tris = 0;
        self.remake = true;
    }

    /// Draw all the paths.
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

    /// Set new window size.
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.ortho_proj = Self::ortho(width, height);
        self.remake = true;
        self.window_size = [gl!(width), gl!(height)];
    }

    // orthographic projection based on the window size, maps pixels to OpenGL normalized coords.
    fn ortho(width: u32, height: u32) -> [GLfloat; 16] {
        [
            TWO / gl!(width),  ZERO,              ZERO, ZERO,
            ZERO,              TWO / gl!(height), ZERO, ZERO,
            ZERO,              ZERO,              ONE,  ZERO,
            -ONE,             -ONE,               ZERO, ONE
        ]
    }
}

impl<'a, W: Window> Drop for Drawing<'a, W> {
    /// Clean up all OpenGL stuff on drop.
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

// read text from a file into a string.
fn read_file(file_name: &str) -> Result<String, TrdlError> {
    let mut contents = String::new();
    let mut f = try!(File::open(file_name));
    try!(f.read_to_string(&mut contents));
    Ok(contents)
}

// Choose control points to represent a straight line as a Bezier curve.
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

// determine if the edge of a triangle is also an exterior edge of the polygon.
fn triangle_edges(i0: usize, i1: usize, i2: usize, max: usize) -> (bool, bool, bool) {
    let e2 = i1 == 0 && i0 == max || (i1 > i0 && i1 - i0 == 1);
    let e0 = i2 == 0 && i1 == max || (i2 > i1 && i2 - i1 == 1);
    let e1 = i0 == 0 && i2 == max || (i0 > i2 && i0 - i2 == 1);
    (e0, e1, e2)
}
