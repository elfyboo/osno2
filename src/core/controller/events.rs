#[derive(Debug, Clone)]
pub enum EventType {
    NewEvent,
    EventHandled,
    EventError,
}

#[derive(Debug, Clone)]
pub struct Events {
    pub event_buffer: Vec<EventType>,
}
