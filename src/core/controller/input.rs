#[derive(Clone, Debug)]
pub enum MouseButtons {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Debug)]
pub enum InputType {
    Mouse(i64, i64),
    MouseButton(MouseButtons),
    MouseWheel(i64),
    Keyboard(char),
}

#[derive(Debug, Clone)]
pub struct Input {
    pub input_buffer: Vec<InputType>,
}
