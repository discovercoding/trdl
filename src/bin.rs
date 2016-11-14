extern crate glutin;
extern crate trdl;

use std::os::raw::c_void;

fn off(point: (f32, f32), offset: (f32, f32)) -> (f32, f32) {
    (point.0 + offset.0, point.1 + offset.1)
}

fn make_shape(off_x: f32, off_y: f32) -> Vec<trdl::Path> {
    let o = (off_x, off_y);
    let mut shapes = Vec::new();
    let foot = trdl::Path::new(off((200f32, -100f32), o)).
        line_to(off((100f32, -200f32), o)).
        curve_to(off((125f32, -300f32), o), off((200f32, -350f32), o), off((300f32, -300f32), o)).
        curve_to(off((280f32, -200f32), o), off((250f32, -150f32), o), off((200f32, -100f32), o)).
        close_path().set_stroke(0.0f32, 0.0f32, 0.0f32, 2).set_fill_color(0.0f32, 1.0f32, 0.0f32);
    shapes.push(foot);
    shapes
}

struct Window {
    w: glutin::Window
}

impl trdl::Window for Window {
    fn set_context(&self) {
        unsafe { self.w.make_current().unwrap() };
    }
    fn load_fn(&self, addr: &str) -> *const c_void {
        self.w.get_proc_address(addr) as *const c_void
    }
 }

fn main() {
    let window_size = (1024, 768);
    let window = Window {
        w: glutin::WindowBuilder::new().
        with_dimensions(window_size.0, window_size.1).
        with_title("TRDL Test").
        build().unwrap() };

    let mut drawing = trdl::Drawing::new(&window, window_size.0, window_size.1,
            0.4, 0.5, 0.6).unwrap();

    let mut idx = 0usize;
    let sqrt_size = 1u32;
    let num_shapes = sqrt_size * sqrt_size;
    let wx = window_size.0 as i32 - 300i32;
    let wy = window_size.1 as i32 - 300i32;

    let colors = [(1.0f32, 0.5f32, 0.0f32), (0.0f32, 0.7f32, 0.4f32),
                  (0.5f32, 0.7f32, 0.3f32), (0.3f32, 0.2f32, 0.9f32)];
    let stroke_colors = [(0.0f32, 0.0f32, 1.0f32), (0.5f32, 0.0f32, 0.2f32),
        (0.0f32, 0.0f32, 0.3f32), (0.3f32, 0.7f32, 0.0f32)];
    let thicknesses = [5, 10, 20, 50];

//    let mut do_fill = true;
//    for i in 0..sqrt_size {
//        let delta_x = 100 + wx * (i as i32) / (sqrt_size as i32);
//        for j in 0..sqrt_size {
//            let delta_y = 100 + wy * (j as i32) / (sqrt_size as i32);
//            let fill_color = if do_fill { Some(colors[idx]) } else { None };
//            let shapes = make_shape(delta_x as f32, delta_y as f32);
//            for s in shapes {
//                drawing.add_path(s).unwrap();
//            }
//            do_fill = !do_fill;
//            idx += 1;
//            if idx > 3 { idx = 0; }
//        }
//    }

    let delta_x = 400f32;
    let delta_y = 400f32;
    let shapes = make_shape(delta_x as f32, delta_y as f32);
    for s in shapes {
        drawing.add_path(s).unwrap();
    }

    drawing.draw();
    window.w.swap_buffers().unwrap();
    for event in window.w.wait_events() {
        if let glutin::Event::Closed = event {
            break;
        }
    }
}