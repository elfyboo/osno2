// controller/mod.rs
pub mod event_bus;
pub mod events;
pub mod input;

//use crossbeam_channel::Sender;
use tokio::task::JoinHandle;

use crate::core::controller::event_bus::EventBus;
//use crate::core::controller::events::AppEvent;

pub struct Controller {
    pub bus: EventBus,
    input_task: JoinHandle<()>,
}

impl Controller {
    pub fn spawn() -> Self {
        let bus = EventBus::new();
        let input_task = tokio::spawn(input::poll_loop(bus.sender()));
        Self { bus, input_task }
    }

    pub fn stop(self) {
        self.input_task.abort();
    }
}
