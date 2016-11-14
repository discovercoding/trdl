extern crate glutin;
extern crate trdl;

use std::os::raw::c_void;

fn offset(point: (f32, f32), offset: (f32, f32)) -> (f32, f32) {
    (point.0 + offset.0, point.1 + offset.1)
}

fn make_foot(off: (f32, f32), is_right: bool, is_back: bool) -> trdl::Path {
    let mut points = [(0f32, 0f32), (50f32, 50f32), (80f32, 100f32), (100f32, 200f32), 
                     (0f32, 250f32), (-75f32, 200f32), (-100f32, 100f32), (0f32, 0f32)];
    let stroke = (0f32, 0f32, 0f32, 6);
    let fill_color = (0f32, 1f32, 0f32);
    let mut backwards = false;
    if is_right {
        if is_back {
            for p in &mut points {
                *p = (-p.0 + off.0, -p.1 + off.1);
            }
        } else {
            for p in &mut points {
                *p = (p.0 + off.0, -p.1 + off.1);
            }
            backwards = true;
        }
    } else {
        if is_back {
            for p in &mut points {
                *p = (-p.0 + off.0, p.1 + off.1);
            }
            backwards = true;
        } else {
            for p in &mut points {
                *p = (p.0 + off.0, p.1 + off.1);
            }
        }
    }
    if backwards {
        trdl::Path::new(points[7]).
            line_to(points[6]).
            curve_to(points[5], points[4], points[3]).
            curve_to(points[2], points[1], points[0]).
            close_path().
            set_stroke(stroke.0, stroke.1, stroke.2, stroke.3).
            set_fill_color(fill_color.0, fill_color.1, fill_color.2)
    } else {
        trdl::Path::new(points[0]).
            curve_to(points[1], points[2], points[3]).
            curve_to(points[4], points[5], points[6]).
            line_to(points[7]).
            close_path().
            set_stroke(stroke.0, stroke.1, stroke.2, stroke.3).
            set_fill_color(fill_color.0, fill_color.1, fill_color.2)
    }
}

fn make_shape(off_x: f32, off_y: f32) -> Vec<trdl::Path> {
    let mut paths = Vec::new();
    paths.push(make_foot((off_x + 200f32, off_y + 100_f32), false, false));
    paths.push(make_foot((off_x + 200f32, off_y - 100_f32), true, false));
    paths.push(make_foot((off_x - 200f32, off_y + 80_f32), false, true));
    paths.push(make_foot((off_x - 200f32, off_y - 80_f32), true, true));
    paths
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
    let window_size = (1280, 800);
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

    let paths = make_shape(600f32, 400f32);
    for p in paths {
        drawing.add_path(p).unwrap();
    }

    drawing.draw();
    window.w.swap_buffers().unwrap();
    for event in window.w.wait_events() {
        if let glutin::Event::Closed = event {
            break;
        }
    }
}