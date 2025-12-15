use shared::math::Vec2;

#[repr(C)]
#[derive(Copy, Clone)]
pub enum InputAction {
    LeftClick,
    RightClick,
    Q,
    W,
    E,
    R,

    SwitchCameraMode,
    CameraFollow,
}

impl InputAction {
    pub fn get_value(self) -> u32 {
        1 << (self as u32)
    }
}

pub struct InputState {
    state: u32,
    pressed_events: u32,
    released_events: u32,
    mouse_position: Vec2,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            state: 0,
            pressed_events: 0,
            released_events: 0,
            mouse_position: Vec2::ZERO,
        }
    }

    pub fn set_action(&mut self, action: InputAction, value: bool) {
        let bit = action.get_value();
        let was_down = (self.state & bit) != 0;

        if value {
            self.state |= bit;
            if !was_down {
                self.pressed_events |= bit;
            }
        } else {
            self.state &= !bit;
            if was_down {
                self.released_events |= bit;
            }
        }
    }

    pub fn reset(&mut self) {
        self.pressed_events = 0;
        self.released_events = 0;
    }

    pub fn is_pressed(&self, action: InputAction) -> bool {
        (self.pressed_events & action.get_value()) != 0
    }

    #[allow(dead_code)]
    pub fn is_released(&self, action: InputAction) -> bool {
        (self.released_events & action.get_value()) != 0
    }

    pub fn is_down(&self, action: InputAction) -> bool {
        (self.state & action.get_value()) != 0
    }

    pub fn set_mouse_position(&mut self, position: Vec2) {
        self.mouse_position = position;
    }

    pub fn get_mouse_position(&self) -> Vec2 {
        self.mouse_position
    }
}
