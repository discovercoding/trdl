extern crate glutin;
extern crate trdl;

use std::os::raw::c_void;

fn make_shape(off_x: f32, off_y: f32, fill_color: Option<(f32, f32, f32)>,
              stroke_color: (f32, f32, f32), stroke_width: u32) -> trdl::Path {
    trdl::Path::new((500f32, 300f32)).
        arc_to(200f32, 200f32, 0f32, (300f32, 500f32), true, true).
        arc_to(200f32, 200f32, 0f32, (500f32, 300f32), false, true).
        //arc_to(200f32, 200f32, 0f32, (100f32, 300f32), true, true).
        //arc_to(200f32, 200f32, 0f32, (300f32, 100f32), true, true).
        //arc_to(200f32, 200f32, 0f32, (500f32, 300f32), true, true).
        set_stroke(0.1f32, 0.8f32, 0f32, 6)

//    let a0 = (150f32 + off_x, 150f32 + off_y);
//    let b0 = (300f32 + off_x, 200f32 + off_y);
//    let bc1 = (350f32 + off_x, 250f32 + off_y);
//    let bc2 = (200f32 + off_x, 250f32 + off_y);
//    let c0 = (200f32 + off_x, 300f32 + off_y);
//    let d0 = (50f32 + off_x, 100f32 + off_y);
//    let de1 = (0f32 + off_x, 50f32 + off_y);
//    let de2 = (50f32 + off_x, 0f32 + off_y);
//    let e0 = (150f32 + off_x, 0f32 + off_y);
//
//    trdl::Path::with_num_vertices(a0, 5).
//        line_to(b0).
//        curve_to(bc1, bc2, c0).
//        line_to(d0).
//        curve_to(de1, de2, e0).
//        // line_to(a0). // automatic, but allowed
//        close_path().
//        set_stroke(stroke_color.0, stroke_color.1, stroke_color.2, stroke_width)

//    let a = (  0f32 + off_x, 0f32   + off_y);
//    let b = (200f32 + off_x, 0f32   + off_y);
//    let c = (200f32 + off_x, 200f32 + off_y);
//    let d = (  0f32 + off_x, 200f32 + off_y);

//    let path = trdl::Path::with_num_vertices(4).
//        add_straight_line(a).
//        add_straight_line(b).
//        add_straight_line(c).
//        add_straight_line(d).
//        set_stroke(stroke_color.0, stroke_color.1, stroke_color.2, stroke_width);
//    if let Some(fill_color) = fill_color {
//        path.set_fill_color(fill_color.0, fill_color.1, fill_color.2).close_path()
//    } else {
//        path
//    }
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

    let mut do_fill = true;
    for i in 0..sqrt_size {
        let delta_x = 100 + wx * (i as i32) / (sqrt_size as i32);
        for j in 0..sqrt_size {
            let delta_y = 100 + wy * (j as i32) / (sqrt_size as i32);
            let fill_color = if do_fill { Some(colors[idx]) } else { None };
            drawing.add_path
                (make_shape(delta_x as f32, delta_y as f32,
                            fill_color, stroke_colors[idx], thicknesses[idx])).unwrap();
            do_fill = !do_fill;
            idx += 1;
            if idx > 3 { idx = 0; }
        }
    }
    drawing.draw();
    window.w.swap_buffers().unwrap();
    for event in window.w.wait_events() {
        if let glutin::Event::Closed = event {
            break;
        }
    }
}