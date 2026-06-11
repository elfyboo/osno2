#[derive(Debug, Clone)]
pub struct EventBus {
    pub event_queue: Vec<String>,
    pub subscribers: Vec<String>,
}
