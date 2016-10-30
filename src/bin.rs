extern crate glutin;
extern crate trdl;

use std::os::raw::c_void;

fn make_shape(off_x: f32, off_y: f32) -> (trdl::FilledPath) {
    let a0 = (150f32 + off_x, 150f32 + off_y);
    let b0 = (300f32 + off_x, 200f32 + off_y);
    let bc1 = (350f32 + off_x, 250f32 + off_y);
    let bc2 = (200f32 + off_x, 250f32 + off_y);
    let c0 = (200f32 + off_x, 300f32 + off_y);
    let d0 = (50f32 + off_x, 100f32 + off_y);
    let de1 = (0f32 + off_x, 50f32 + off_y);
    let de2 = (50f32 + off_x, 0f32 + off_y);
    let e0 = (150f32 + off_x, 0f32 + off_y);

    trdl::FilledPath::with_num_vertices(5).
        add_straight_line(a0).
        add_bezier_curve(b0, bc1, bc2).
        add_straight_line(c0).
        add_bezier_curve(d0, de1, de2).
        add_straight_line(e0).
        set_fill_color(1f32, 1f32, 0f32).
        set_stroke(0f32, 0f32, 1f32, 2)
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

    let mut drawing = trdl::Drawing::new(&window, window_size.0, window_size.1);

    let mut depth_idx = 0u32;
    let sqrt_size = 10u32;
    let num_shapes = sqrt_size * sqrt_size;
    let wx = window_size.0 as i32 - 300i32;
    let wy = window_size.1 as i32 - 300i32;
    for i in 0..sqrt_size {
        let delta_x = wx * (i as i32) / (sqrt_size as i32);
        for j in 0..sqrt_size {
            let delta_y = wy * (j as i32) / (sqrt_size as i32);
            let depth = 1.0f32 - 2.0f32 * (depth_idx as f32) / (num_shapes as f32);
            depth_idx += 1;
            drawing.add_filled_path(make_shape(delta_x as f32, delta_y as f32), depth).unwrap();
        }
    }
    drawing.draw();
    for event in window.w.wait_events() {
        if let glutin::Event::Closed = event {
            break;
        }
        drawing.draw();
        window.w.swap_buffers().unwrap();
    }
}