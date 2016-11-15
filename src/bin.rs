extern crate glutin;
extern crate trdl;

use std::f32;
use std::os::raw::c_void;

fn offset(point: (f32, f32), offset: (f32, f32)) -> (f32, f32) {
    (point.0 + offset.0, point.1 + offset.1)
}

fn make_foot(off: (f32, f32), is_right: bool, is_back: bool) -> Vec<trdl::Path> {
    let mut foot_points = [(0f32, 0f32), (50f32, 50f32), (80f32, 100f32), (100f32, 200f32),
        (0f32, 250f32), (-75f32, 200f32), (-100f32, 100f32), (0f32, 0f32)];
    let mut toe_points = [(100f32, 200f32), (75f32, 210f32), (50f32, 215f32)];

    let stroke = (0f32, 0f32, 0f32, 6);
    let fill_color = (0.684f32, 0.765f32, 0f32);
    let mut backwards = false;
    if is_right {
        if is_back {
            for p in &mut foot_points {
                *p = (-p.0 + off.0, -p.1 + off.1);
            }
            for p in &mut toe_points {
                *p = (-p.0 + off.0, -p.1 + off.1);
            }
        } else {
            for p in &mut foot_points {
                *p = (p.0 + off.0, -p.1 + off.1);
            }
            for p in &mut toe_points {
                *p = (p.0 + off.0, -p.1 + off.1);
            }
            backwards = true;
        }
    } else {
        if is_back {
            for p in &mut foot_points {
                *p = (-p.0 + off.0, p.1 + off.1);
            }
            for p in &mut toe_points {
                *p = (-p.0 + off.0, p.1 + off.1);
            }
            backwards = true;
        } else {
            for p in &mut foot_points {
                *p = (p.0 + off.0, p.1 + off.1);
            }
            for p in &mut toe_points {
                *p = (p.0 + off.0, p.1 + off.1);
            }
        }
    }
    if backwards {
        vec![trdl::Path::new(foot_points[7]).
             line_to(foot_points[6]).
             curve_to(foot_points[5], foot_points[4], foot_points[3]).
             curve_to(foot_points[2], foot_points[1], foot_points[0]).
             close_path().
             set_stroke(stroke.0, stroke.1, stroke.2, stroke.3).
             set_fill_color(fill_color.0, fill_color.1, fill_color.2),
             trdl::Path::ellipse(toe_points[0], 10f32, 10f32, 0f32).
                 set_fill_color(0.718f32, 0.51f32, 0f32).set_stroke(0f32, 0f32, 0f32, 2),
             trdl::Path::ellipse(toe_points[1], 10f32, 10f32, 0f32).
                 set_fill_color(0.718f32, 0.51f32, 0f32).set_stroke(0f32, 0f32, 0f32, 2),
             trdl::Path::ellipse(toe_points[2], 10f32, 10f32, 0f32).
                 set_fill_color(0.718f32, 0.51f32, 0f32).set_stroke(0f32, 0f32, 0f32, 2)]
    } else {
        vec![trdl::Path::new(foot_points[0]).
             curve_to(foot_points[1], foot_points[2], foot_points[3]).
             curve_to(foot_points[4], foot_points[5], foot_points[6]).
             line_to(foot_points[7]).
             close_path().
             set_stroke(stroke.0, stroke.1, stroke.2, stroke.3).
             set_fill_color(fill_color.0, fill_color.1, fill_color.2),
             trdl::Path::ellipse(toe_points[0], 10f32, 10f32, 0f32).
                 set_fill_color(0.718f32, 0.51f32, 0f32).set_stroke(0f32, 0f32, 0f32, 2),
             trdl::Path::ellipse(toe_points[1], 10f32, 10f32, 0f32).
                 set_fill_color(0.718f32, 0.51f32, 0f32).set_stroke(0f32, 0f32, 0f32, 2),
             trdl::Path::ellipse(toe_points[2], 10f32, 10f32, 0f32).
                 set_fill_color(0.718f32, 0.51f32, 0f32).set_stroke(0f32, 0f32, 0f32, 2)]
    }
}

fn make_head(off: (f32, f32)) -> Vec<trdl::Path> {
    vec![trdl::Path::ellipse(off, 135f32, 90f32, 0f32).
            set_fill_color(0.684f32, 0.765f32, 0f32).set_stroke(0f32, 0f32, 0f32, 6),
         trdl::Path::ellipse((off.0 + 100f32, off.1 + 55f32), 10f32, 10f32, 0f32).
             set_fill_color(0f32, 0f32, 0f32),
         trdl::Path::ellipse((off.0 + 100f32, off.1 - 55f32), 10f32, 10f32, 0f32).
             set_fill_color(0f32, 0f32, 0f32)]
}

fn make_tail(off: (f32, f32)) -> trdl::Path {
    trdl::Path::new(offset((50f32, 20f32), off)).
        curve_to(offset((0f32, 20f32), off), offset((-50f32, 80f32), off),
                 offset((-100f32, 80f32), off)).
        curve_to(offset((-50f32, 50f32), off), offset((-50f32, -30f32), off),
                 offset((50f32, -20f32), off)).close_path().
        set_fill_color(0.684f32, 0.765f32, 0f32).set_stroke(0f32, 0f32, 0f32, 6)
}

fn make_shell(off: (f32, f32)) -> Vec<trdl::Path> {
    let radius = 250f32;
    let c = f32::consts::FRAC_PI_3.cos();
    let s = f32::consts::FRAC_PI_3.sin();
    let cr = c * radius;
    let sr = s * radius;
    let hex_rad = 175f32;
    let chr = c * hex_rad;
    let shr = s * hex_rad;
    vec![trdl::Path::ellipse(off, radius, radius, 0f32).
         set_fill_color(0.454f32, 0.522f32, 0f32).set_stroke(0f32, 0f32, 0f32, 6),
         trdl::Path::new((off.0, off.1 + radius)).line_to((off.0, off.1 - radius)).
             set_stroke(0f32, 0f32, 0f32, 6),
         trdl::Path::new((off.0 + sr, off.1 + cr)).line_to((off.0 - sr, off.1 - cr)).
             set_stroke(0f32, 0f32, 0f32, 6),
         trdl::Path::new((off.0 + sr, off.1 - cr)).line_to((off.0 - sr, off.1 + cr)).
             set_stroke(0f32, 0f32, 0f32, 6),
         trdl::Path::new((off.0, off.1 - hex_rad)).
             line_to((off.0 + shr, off.1 - chr)).
             line_to((off.0 + shr, off.1 + chr)).
             line_to((off.0, off.1 + hex_rad)).
             line_to((off.0 - shr, off.1 + chr)).
             line_to((off.0 - shr, off.1 - chr)).close_path().
             set_fill_color(0.454f32, 0.682f32, 0f32).set_stroke(0f32, 0f32, 0f32, 6)]

}

fn make_shape(off_x: f32, off_y: f32) -> Vec<trdl::Path> {
    let mut paths = Vec::new();
    paths.append(&mut make_foot((off_x + 220f32, off_y + 100_f32), false, false));
    paths.append(&mut make_foot((off_x + 220f32, off_y - 100_f32), true, false));
    paths.append(&mut make_foot((off_x - 200f32, off_y + 80_f32), false, true));
    paths.append(&mut make_foot((off_x - 200f32, off_y - 80_f32), true, true));
    paths.append(&mut make_head((off_x + 320f32, off_y)));
    paths.push(make_tail((off_x - 275f32, off_y)));
    paths.append(&mut make_shell((off_x, off_y)));
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