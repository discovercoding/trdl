extern crate gl;
extern crate glutin;
extern crate trdl;

use std::mem;
use std::ffi::CString;
use std::ptr;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use gl::types::*;
use trdl::drawing_gl::shader::*;
use trdl::triangulation::triangulate;
use trdl::triangulation::find_edges;

fn read_file(file_name: &str) -> io::Result<String> {
    let mut contents = String::new();
    let mut f = try!(File::open(file_name));
    try!(f.read_to_string(&mut contents));
    Ok(contents)
}

fn bezier_line_control_points(first: (GLfloat, GLfloat), last: (GLfloat, GLfloat))-> 
        ((GLfloat, GLfloat), (GLfloat, GLfloat)) {
    let dx = (last.0 - first.0) / 3.0 as GLfloat;
    let dy = (last.1 - first.1) / 3.0 as GLfloat;

    let v1 = (first.0 + dx, first.1 + dy);
    (v1, (v1.0 + dx, v1.1 + dy))
}

fn make_shape() -> ([GLfloat; 27], [GLfloat; 18], [GLfloat; 18], [GLfloat; 27], [GLuint; 9]) {
    let a0  = (0.15 as GLfloat, 0.30 as GLfloat);
    let ab1 = (0.05 as GLfloat, 0.30 as GLfloat);
    let ab2 = (0.00 as GLfloat, 0.25 as GLfloat);
    let b0  = (0.05 as GLfloat, 0.20 as GLfloat);
    let c0  = (0.20 as GLfloat, 0.00 as GLfloat);
    let cd1 = (0.20 as GLfloat, 0.05 as GLfloat);
    let cd2 = (0.35 as GLfloat, 0.05 as GLfloat);
    let d0  = (0.30 as GLfloat, 0.10 as GLfloat);
    let e0  = (0.15 as GLfloat, 0.15 as GLfloat);

    let (bc1, bc2) = bezier_line_control_points(b0, c0);
    let (be1, be2) = bezier_line_control_points(b0, e0);
    let (ce1, ce2) = bezier_line_control_points(c0, e0);
    let (de1, de2) = bezier_line_control_points(d0, e0);
    let (ea1, ea2) = bezier_line_control_points(e0, a0);

    let polygon = vec![a0, b0, c0, d0, e0];
    let indices = triangulate(&polygon).unwrap();
    let edges = find_edges(&indices, polygon.len());

    ([a0.0, a0.1, 0.0 as GLfloat,
      b0.0, b0.1, 0.0 as GLfloat,
      e0.0, e0.1, 0.0 as GLfloat,
      b0.0, b0.1, 0.0 as GLfloat,
      c0.0, c0.1, 0.0 as GLfloat,
      e0.0, e0.1, 0.0 as GLfloat,
      e0.0, e0.1, 0.0 as GLfloat,
      c0.0, c0.1, 0.0 as GLfloat,
      d0.0, d0.1, 0.0 as GLfloat],

     [ab1.0, ab1.1,
      be1.0, be1.1,
      ea1.0, ea1.1,
      bc1.0, bc1.1,
      ce1.0, ce1.1,
      be2.0, be2.1,
      ce2.0, ce2.1,
      cd1.0, cd1.1,
      de1.0, de1.1],

     [ab2.0, ab2.1,
      be2.0, be2.1,
      ea2.0, ea2.1,
      bc2.0, bc2.1,
      ce2.0, ce2.1,
      be1.0, be1.1,
      ce1.0, ce1.1,
      cd2.0, cd2.1,
      de2.0, de2.1],

     [1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat,
      1.0 as GLfloat, 1.0 as GLfloat, 0.0 as GLfloat],
      
     [0 as GLuint,
      1 as GLuint,
      1 as GLuint,
      0 as GLuint,
      0 as GLuint,
      1 as GLuint,
      1 as GLuint,
      1 as GLuint,
      0 as GLuint])
}

fn main() {
    let window = glutin::Window::new().unwrap();
    unsafe { window.make_current().unwrap() };
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let (position, control_1, control_2, color, edge) = make_shape();

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

        println!("program_id={}, in_position={}, in_control_1={}, in_control_2={}, in_color={}, in_edge={}",
            program_id, in_position, in_control_1, in_control_2, in_color, in_edge);

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

        println!("position_vbo={}, control_1_vbo={}, control_2_vbo={}, color_vbo={}, edge_vbo={}",
            position_vbo, control_1_vbo, control_2_vbo, color_vbo, edge_vbo);

        // Populate the position buffer
        println!("position[0..2] = [{}, {}, {}], len = {}", position[0], position[1], position[2], position.len()); 
        gl::BindBuffer(gl::ARRAY_BUFFER, position_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (position.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&position[0]),
                       gl::STATIC_DRAW);

        // Populate the control points buffers
        println!("control_1[0..2] = [{}, {}, {}], len = {}", control_1[0], control_1[1], control_1[2], control_1.len()); 
        gl::BindBuffer(gl::ARRAY_BUFFER, control_1_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (control_1.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&control_1[0]),
                       gl::STATIC_DRAW);

        // Populate the control points buffers
        println!("control_2[0..2] = [{}, {}, {}], len = {}", control_2[0], control_2[1], control_2[2], control_2.len()); 
        gl::BindBuffer(gl::ARRAY_BUFFER, control_2_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (control_2.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&control_2[0]),
                       gl::STATIC_DRAW);

        // Populate color buffer
        println!("color[0..2] = [{}, {}, {}], len = {}", color[0], color[1], color[2], color.len()); 
        gl::BindBuffer(gl::ARRAY_BUFFER, color_vbo);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (color.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                       mem::transmute(&color[0]),
                       gl::STATIC_DRAW);

        // Populate the edge buffer
        println!("edge[0..2] = [{}, {}, {}], len = {}", edge[0], edge[1], edge[2], edge.len()); 
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

        gl::UseProgram(program_id);

        if outer_tess_uniform >= 0 {
            gl::Uniform1i(outer_tess_uniform, 32);
            println!("outer_tess good!");
        }

        if inner_tess_uniform >= 0 {
            gl::Uniform1i(inner_tess_uniform, 1);
            println!("inner_tess good!");
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
            gl::DrawArrays(gl::PATCHES, 0, 9);

            window.swap_buffers().unwrap();
        }

        gl::DeleteBuffers(NUM_VBO, mem::transmute(&vbo_handles[0]));
        gl::DeleteVertexArrays(1, &vao_handle);
    }
}