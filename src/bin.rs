extern crate gl;
extern crate glutin;
extern crate trdl;
extern crate time;

use std::mem;
use std::ffi::CString;
use std::ptr;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::collections::hash_map::HashMap;
use gl::types::*;
use trdl::drawing_gl::shader::*;
use trdl::triangulation::triangulate;
use time::precise_time_s;

// this just saves us from writing "as GLfloat" in common cases
const ZERO:  GLfloat = 0.0 as GLfloat;
const ONE:   GLfloat = 1.0 as GLfloat;
const TWO:   GLfloat = 2.0 as GLfloat;
const THREE: GLfloat = 3.0 as GLfloat;

fn read_file(file_name: &str) -> io::Result<String> {
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

fn handle_vertex_pair(polygon: &Vec<(GLfloat, GLfloat)>, i0: usize, i1: usize, depth: GLfloat,
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

fn push_colors(cs: &mut Vec<GLfloat>) {
    cs.push(ONE);
    cs.push(ONE);
    cs.push(ZERO);
    
    cs.push(ONE);
    cs.push(ONE);
    cs.push(ZERO);

    cs.push(ONE);
    cs.push(ONE);
    cs.push(ZERO);
}

fn triangle_edges(i0: usize, i1: usize, i2: usize, max: usize) -> (bool, bool, bool) { 
    let e2 = i1 == 0 && i0 == max || (i1 > i0 && i1 - i0 == 1);
    let e0 = i2 == 0 && i1 == max || (i2 > i1 && i2 - i1 == 1);
    let e1 = i0 == 0 && i2 == max || (i2 > i2 && i0 - i2 == 1);
    (e0, e1, e2)
}

fn make_shape(off_x: GLfloat, off_y: GLfloat, depth: GLfloat) -> (Vec<GLfloat>, Vec<GLfloat>, Vec<GLfloat>, Vec<GLfloat>, Vec<GLuint>) {
    //let a0  = (0.15 as GLfloat + off_x, -0.30 as GLfloat + off_y);
    //let ab2 = (0.05 as GLfloat + off_x, -0.30 as GLfloat + off_y);
    //let ab1 = (0.00 as GLfloat + off_x, -0.25 as GLfloat + off_y);
    //let b0  = (0.05 as GLfloat + off_x, -0.20 as GLfloat + off_y);
    //let c0  = (0.20 as GLfloat + off_x, -0.00 as GLfloat + off_y);
    //let cd2 = (0.20 as GLfloat + off_x, -0.05 as GLfloat + off_y);
    //let cd1 = (0.35 as GLfloat + off_x, -0.05 as GLfloat + off_y);
    //let d0  = (0.30 as GLfloat + off_x, -0.10 as GLfloat + off_y);
    //let e0  = (0.15 as GLfloat + off_x, -0.15 as GLfloat + off_y);

    let a0  = (150 as GLfloat,   0 as GLfloat);
    let ab2 = ( 50 as GLfloat,   0 as GLfloat);
    let ab1 = (  0 as GLfloat,  50 as GLfloat);
    let b0  = ( 50 as GLfloat, 100 as GLfloat);
    let c0  = (200 as GLfloat, 300 as GLfloat);
    let cd2 = (200 as GLfloat, 250 as GLfloat);
    let cd1 = (350 as GLfloat, 250 as GLfloat);
    let d0  = (300 as GLfloat, 200 as GLfloat);
    let e0  = (150 as GLfloat, 150 as GLfloat);
                
    let mut control_point_map = HashMap::new();
    control_point_map.insert((3, 4), (ab1, ab2));
    control_point_map.insert((1, 2), (cd1, cd2));

    let polygon = vec![e0, d0, c0, b0, a0];
    let indices = triangulate(&polygon).unwrap();
    let num_tris = indices.len() / 3;

    let mut vs = Vec::with_capacity(3*num_tris);
    let mut cp1s = Vec::with_capacity(2*num_tris);
    let mut cp2s = Vec::with_capacity(2*num_tris);
    let mut cs = Vec::with_capacity(3*num_tris);
    let mut es = Vec::with_capacity(num_tris);
    let num_verts = polygon.len();
    for t in 0..num_tris {
        let ti0 = 3*t;
        let ti1 = ti0+1;
        let ti2 = ti1+1;
        handle_vertex_pair(&polygon, indices[ti0], indices[ti1], depth, &mut control_point_map, &mut vs, &mut cp1s, &mut cp2s);
        handle_vertex_pair(&polygon, indices[ti1], indices[ti2], depth, &mut control_point_map, &mut vs, &mut cp1s, &mut cp2s);
        handle_vertex_pair(&polygon, indices[ti2], indices[ti0], depth, &mut control_point_map, &mut vs, &mut cp1s, &mut cp2s);
        push_colors(&mut cs);
        let (e0, e1, e2) = triangle_edges(ti0, ti1, ti2, num_verts);
        es.push(e0 as GLuint);
        es.push(e1 as GLuint);
        es.push(e2 as GLuint);
     }
     (vs, cp1s, cp2s, cs, es)
}

fn make_shapes(sqrt_size: usize) -> (Vec<GLfloat>, Vec<GLfloat>, Vec<GLfloat>, Vec<GLfloat>, Vec<GLuint>) {
    let num_shapes = sqrt_size * sqrt_size;

    let mut vertex_vec = Vec::new();
    let mut cp1_vec = Vec::new();
    let mut cp2_vec = Vec::new();
    let mut color_vec = Vec::new();
    let mut edge_vec = Vec::new();

    let mut depth_idx = 0;
    for i in 0..sqrt_size {
        let delta_x = ((2*i) as GLfloat) / (80 as GLfloat) - ONE;
        for j in 0..sqrt_size {
            let delta_y = ONE - ((2*j) as GLfloat) / (85 as GLfloat);
            let depth = ONE - ((2 * depth_idx) as GLfloat) / (num_shapes as GLfloat);
            depth_idx += 1;

            let (mut vs, mut cp1s, mut cp2s, mut cs, mut es) = make_shape(delta_x, delta_y, depth);
            vertex_vec.append(&mut vs);
            cp1_vec.append(&mut cp1s);
            cp2_vec.append(&mut cp2s);
            color_vec.append(&mut cs);
            edge_vec.append(&mut es);
        }
    }
    (vertex_vec, cp1_vec, cp2_vec, color_vec, edge_vec)
}

fn main() {
    let window_size = (1024, 768);
    let window = 
        glutin::WindowBuilder::new().
        with_dimensions(window_size.0, window_size.1).
        with_title("TRDL Test").
        build().unwrap();

    let ortho_proj = [
        TWO / window_size.0 as GLfloat,  ZERO, ZERO, ZERO,
        ZERO, TWO / window_size.1 as GLfloat, ZERO, ZERO,
        ZERO, ZERO, ONE, ZERO,
        -ONE, -ONE, ZERO, ONE
    ];

    unsafe { window.make_current().unwrap() };
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let vertex_shader_code = read_file("vertex_shader.glsl").unwrap();
    let tess_control_shader_code = read_file("tess_control_shader.glsl").unwrap();
    let tess_evaluation_shader_code = read_file("tess_evaluation_shader.glsl").unwrap();
    let geometry_shader_code = read_file("geometry_shader.glsl").unwrap();
    let fragment_shader_code = read_file("fragment_shader.glsl").unwrap();
    let program;
    {
        let mut builder = ShaderProgramBuilder::new();
        builder.set_vertex_shader(&vertex_shader_code);
        builder.set_tess_control_shader(&tess_control_shader_code);
        builder.set_tess_evaluation_shader(&tess_evaluation_shader_code);
        builder.set_geometry_shader(&geometry_shader_code);
        builder.set_fragment_shader(&fragment_shader_code);
        program = builder.build_shader_program().unwrap();
    }
    let program_id = program.get_program_id();

    let begin = precise_time_s();

    let (position, control_1, control_2, color, edge) = make_shapes(2);
    let num_tris = position.len() / 9;

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

        let mut vao_handle = 0 as GLuint;

        // Create the buffer objects
        const NUM_VBO: i32 = 5;
        let vbo_handles = [0 as GLuint, 0 as GLuint, 0 as GLuint, 0 as GLuint, 0 as GLuint];
        gl::GenBuffers(NUM_VBO, mem::transmute(&vbo_handles[0]));

        let position_vbo = vbo_handles[0];
        let control_1_vbo = vbo_handles[1];
        let control_2_vbo = vbo_handles[2];
        let color_vbo = vbo_handles[3];
        let edge_vbo = vbo_handles[4];

        // Populate the position buffer
        gl::BindBuffer(gl::ARRAY_BUFFER, position_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (position.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&position[0]),
                       gl::STATIC_DRAW);

        // Populate the control points buffers
        gl::BindBuffer(gl::ARRAY_BUFFER, control_1_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (control_1.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&control_1[0]),
                       gl::STATIC_DRAW);

        // Populate the control points buffers
        gl::BindBuffer(gl::ARRAY_BUFFER, control_2_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (control_2.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&control_2[0]),
                       gl::STATIC_DRAW);

        // Populate color buffer
        gl::BindBuffer(gl::ARRAY_BUFFER, color_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (color.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&color[0]),
                       gl::STATIC_DRAW);

        // Populate the edge buffer
        gl::BindBuffer(gl::ARRAY_BUFFER, edge_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (edge.len() * mem::size_of::<GLuint>()) as GLsizeiptr,
                       mem::transmute(&edge[0]),
                       gl::STATIC_DRAW);

        gl::PatchParameteri(gl::PATCH_VERTICES, 3);

        // Create and set-up the vertex array object 
        gl::GenVertexArrays(1, &mut vao_handle);
        gl::BindVertexArray(vao_handle);

        // Enable the vertex attribute arrays 
        gl::EnableVertexAttribArray(0 as GLuint); // position
        gl::EnableVertexAttribArray(1 as GLuint); // control point 1
        gl::EnableVertexAttribArray(2 as GLuint); // control point 2
        gl::EnableVertexAttribArray(3 as GLuint); // color
        gl::EnableVertexAttribArray(4 as GLuint); // edge

        gl::BindBuffer(gl::ARRAY_BUFFER, position_vbo);
        gl::VertexAttribPointer(in_position as GLuint, 3, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());
        gl::BindBuffer(gl::ARRAY_BUFFER, control_1_vbo);
        gl::VertexAttribPointer(in_control_1 as GLuint, 2, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());
        gl::BindBuffer(gl::ARRAY_BUFFER, control_2_vbo);
        gl::VertexAttribPointer(in_control_2 as GLuint, 2, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());
        gl::BindBuffer(gl::ARRAY_BUFFER, color_vbo);
        gl::VertexAttribPointer(in_color as GLuint, 3, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());
        gl::BindBuffer(gl::ARRAY_BUFFER, edge_vbo);
        gl::VertexAttribPointer(in_edge as GLuint, 1, gl::UNSIGNED_INT,
                                gl::FALSE as GLboolean, 0, ptr::null());


        let c_str = CString::new("outer_tess".as_bytes()).unwrap();
        let outer_tess_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
        let c_str = CString::new("inner_tess".as_bytes()).unwrap();
        let inner_tess_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());
        let c_str = CString::new("projection".as_bytes()).unwrap();
        let projection_uniform = gl::GetUniformLocation(program_id, c_str.as_ptr());

        gl::UseProgram(program_id);

        if outer_tess_uniform >= 0 {
            gl::Uniform1i(outer_tess_uniform, 32);
        }

        if inner_tess_uniform >= 0 {
            gl::Uniform1i(inner_tess_uniform, 1);
        }

        if projection_uniform >= 0 {
            gl::UniformMatrix4fv(projection_uniform, 1, gl::FALSE as GLboolean, mem::transmute(&ortho_proj[0]));
        }

        gl::Enable(gl::DEPTH_TEST);

         gl::ClearColor(0.1, 0.2, 0.1, 1.0);

        for event in window.wait_events() {
            if let glutin::Event::Closed = event {
                break;
            }

            // Clear the screen
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            // Draw a triangle from the 3 vertices
            gl::BindVertexArray(vao_handle);
            gl::DrawArrays(gl::PATCHES, 9, 9 as GLint);

            window.swap_buffers().unwrap();

            let end = precise_time_s();
            println!("elapsed seconds: {}", end - begin);
        }

        gl::DeleteBuffers(NUM_VBO, mem::transmute(&vbo_handles[0]));
        gl::DeleteVertexArrays(1, &vao_handle);
    }
}