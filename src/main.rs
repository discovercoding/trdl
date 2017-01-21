extern crate glutin;
extern crate trdl;
extern crate rand;

use std::f32;
use std::os::raw::c_void;
use rand::StdRng;
use rand::distributions::{IndependentSample, Range};

const MAX_PARTICLES: usize = 500;

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

enum ParticleShape {
    Rectangle{ width: f32, height: f32, angle: f32},
    Circle{ diameter: f32 }
}

struct Particle {
    position: (f32, f32),
    velocity: (f32, f32),
    stroke: ((f32, f32, f32), u32),
    fill_color: (f32, f32, f32),
    shape: ParticleShape
}

impl Particle {
    fn with_random_inputs(rng: &mut StdRng, win_width: u32, win_height: u32) -> Particle {
        let x_pos_range = Range::new(0f32, win_width as f32);
        let y_pos_range = Range::new(0f32, win_height as f32);
        let vel_range = Range::new(-10f32, 10f32);
        let color_range = Range::new(0f32, 1f32);
        let thickness_range = Range::new(1u32, 6u32);
        let shape_range = Range::new(0, 2);
        let dim_range = Range::new(20f32, 100f32);
        let angle_range = Range::new(0f32, 2f32*f32::consts::PI);

        let position = (x_pos_range.ind_sample(rng), y_pos_range.ind_sample(rng));
        let velocity = (vel_range.ind_sample(rng), vel_range.ind_sample(rng));
        let stroke = ((color_range.ind_sample(rng),
                       color_range.ind_sample(rng),
                       color_range.ind_sample(rng)),
                      thickness_range.ind_sample(rng));
        let fill_color = (color_range.ind_sample(rng),
                          color_range.ind_sample(rng),
                          color_range.ind_sample(rng));
        let dim = (dim_range.ind_sample(rng), dim_range.ind_sample(rng));
        let angle = angle_range.ind_sample(rng);
        let shape =  if shape_range.ind_sample(rng) == 0 {
            ParticleShape::Circle { diameter: dim.0 }
        } else {
            ParticleShape::Rectangle { width: dim.0, height: dim.1, angle: angle }
        };
        Particle { position: position, velocity: velocity, stroke: stroke,
            fill_color: fill_color, shape: shape }
    }

    fn update(&mut self) {
        self.position = (self.position.0 + self.velocity.0, self.position.1 + self.velocity.1);
    }

    fn is_in_window(&self, win_width: u32, win_height: u32) -> bool {
        (self.position.0 >= 0f32 && self.position.0 <= (win_width as f32)) &&
            (self.position.1 >= 0f32 && self.position.1 <= (win_height as f32))
    }

    fn get_path(&self) -> trdl::Path {
        match self.shape {
            ParticleShape::Circle{diameter} => {
                let radius = diameter / 2f32;
                trdl::Path::ellipse(self.position, radius, radius, 0f32)
            },
            ParticleShape::Rectangle{width, height, angle} => {
                trdl::Path::rectangle(self.position, width, height, angle)
            }
        }.
            set_stroke((self.stroke.0).0,
                       (self.stroke.0).1,
                       (self.stroke.0).2,
                        self.stroke.1).
            set_fill_color(self.fill_color.0, self.fill_color.1, self.fill_color.2)
    }
}

fn update_particles(particles: &mut Vec<Particle>, rng: &mut StdRng,
                    win_width: u32, win_height: u32) {
    // update the positions of the particles
    for ref mut p in particles.iter_mut() {
        p.update();
    }
    // get rid of particles that go off the screen
    particles.retain(|p| p.is_in_window(win_width, win_height));
    // add a new random particle if there is room
    while particles.len() < MAX_PARTICLES {
        particles.push(Particle::with_random_inputs(rng, win_width, win_height));
    }
}

fn draw_particles<'a, W: trdl::Window + 'a>(drawing: &mut trdl::Drawing<'a, W>,
                                            particles: &Vec<Particle>) {
    drawing.clear_paths();
    for p in particles {
        drawing.add_path(p.get_path()).unwrap();
    }
    drawing.draw();
}

fn main() {
    let monitor = glutin::get_available_monitors().next().unwrap();
    let window = Window {
        w: glutin::WindowBuilder::new().
            with_fullscreen(monitor).
            with_title("TRDL Particle Demo").
            build().unwrap()
    };
    let window_size = window.w.get_inner_size_pixels().unwrap();
    let mut rng = rand::StdRng::new().unwrap();
    let mut drawing = trdl::Drawing::new(&window, window_size.0, window_size.1,
                                         0.0, 0.0, 0.0).unwrap();

    let mut particles = Vec::new();
    loop {
        for event in window.w.poll_events() {
            match event {
                glutin::Event::Closed => return,
                glutin::Event::ReceivedCharacter(c) => {
                    if c == 'q' || c == 'Q' {
                        return;
                    }
                },
                _ => {}
            }
        }
        update_particles(&mut particles, &mut rng, window_size.0, window_size.1);
        draw_particles(&mut drawing, &particles);
        window.w.swap_buffers().unwrap();
    }
}