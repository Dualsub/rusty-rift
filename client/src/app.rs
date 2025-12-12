use std::{ops::Mul, sync::Arc};

use glam::Vec2;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::input::InputState;
use crate::renderer::Renderer;
use crate::{game::Game, input::InputAction};
use shared::physics::PhysicsWorld;

pub struct State {
    pub window: Arc<Window>,
    pub renderer: Renderer,
    pub physics_world: PhysicsWorld,
    pub game: Game,
    pub input_state: InputState,

    pub previous_time: f64,
    pub time_since_fixed: f32,
}

fn get_time() -> f64 {
    let window = wgpu::web_sys::window().unwrap_throw();
    let performance = window.performance().unwrap_throw();
    performance.now() * 0.001
}

impl State {
    const FIXED_TIMESTEP: f32 = 1.0 / 60.0;

    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let mut renderer = Renderer::new(&window).await?;
        let mut physics_world = PhysicsWorld::new();
        let mut game = Game::new();
        let input_state = InputState::new();

        game.initialize(&mut physics_world);
        game.load_resources(&mut renderer);

        Ok(Self {
            window,
            renderer,
            physics_world,
            game,
            input_state,
            previous_time: get_time(),
            time_since_fixed: 0.0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    pub fn update(&mut self, dt: f32, alpha: f32) {
        self.game.update(dt, alpha, &self.input_state);
    }

    pub fn fixed_update(&mut self, dt: f32) {
        self.game.fixed_update(dt, &mut self.physics_world);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.game.render(&mut self.renderer);
        self.window.request_redraw();
        self.renderer.render()
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match code {
            KeyCode::KeyQ => self.input_state.set_action(InputAction::Q, is_pressed),
            KeyCode::KeyW => self.input_state.set_action(InputAction::W, is_pressed),
            KeyCode::KeyE => self.input_state.set_action(InputAction::E, is_pressed),
            KeyCode::KeyR => self.input_state.set_action(InputAction::R, is_pressed),
            _ => {}
        }

        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(State::new(window).await.expect("Unable to create canvas."))
                            .is_ok()
                    )
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        // This is where proxy.send_event() ends up
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    #[allow(unused_mut)]
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = get_time();
                let dt = (now - state.previous_time).clamp(0.0, 1.0 / 10.0).mul(1.0) as f32; // We clamp it to prevent instability
                state.previous_time = now;

                state.time_since_fixed += dt;
                while state.time_since_fixed > State::FIXED_TIMESTEP {
                    state.fixed_update(State::FIXED_TIMESTEP);
                    state.time_since_fixed -= State::FIXED_TIMESTEP;
                }

                let alpha = (state.time_since_fixed / State::FIXED_TIMESTEP).clamp(0.0, 1.0);
                state.update(dt, alpha);

                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }

                state.input_state.reset();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                state
                    .input_state
                    .set_mouse_position(Vec2::new(position.x as f32, position.y as f32));
            }
            _ => {}
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
