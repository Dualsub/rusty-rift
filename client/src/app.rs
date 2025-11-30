use std::time::{Duration, Instant};

pub struct App {
    running: bool,
}

const FIXED_UPDATE_RATE: i32 = 60;
const FIXED_UPDATE_DELTATIME: f32 = 1.0 / (FIXED_UPDATE_RATE as f32);

impl App {
    fn initialize(&self) {}
    fn shutdown(&self) {}
    fn fixed_update(&self, dt: f32) {
        println!("fixed dt: {}", dt);
    }
    fn update(&self, dt: f32) {
        println!("update dt: {}", dt);
    }
    fn render(&self) {}
    fn should_shutdown(&self) -> bool {
        !self.running
    }

    pub fn new() -> App {
        App { running: true }
    }

    pub fn run(&self, args: &Vec<String>) {
        for arg in args {
            println!("{}", arg);
        }

        self.initialize();

        let mut last_time = Instant::now();
        let mut time_since_fixed: f32 = 0.0;
        while !self.should_shutdown() {
            let now = Instant::now();
            let duration = now - last_time;
            let dt: f32 = Duration::as_secs_f32(&duration);
            last_time = now;

            time_since_fixed += dt;
            while time_since_fixed > FIXED_UPDATE_DELTATIME {
                time_since_fixed -= FIXED_UPDATE_DELTATIME;
                self.fixed_update(FIXED_UPDATE_DELTATIME);
            }

            self.update(dt);

            self.render();
        }

        self.shutdown();
    }
}
