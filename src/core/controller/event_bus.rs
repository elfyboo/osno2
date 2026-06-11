// controller/event_bus.rs
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::{HashMap, HashSet};

use crate::core::controller::events::{AppEvent, EventKind};
use crate::core::model::reducer::reduce;
use crate::core::model::state::AppState;

pub type WidgetId = u32;

/// Synchronous, ordered event dispatcher. Owns the receiving end of
/// the input/event channel; not Clone — share via reference or Arc.
pub struct EventBus {
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    subscribers: HashMap<EventKind, HashSet<WidgetId>>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, rx) = unbounded();
        Self {
            tx,
            rx,
            subscribers: HashMap::new(),
        }
    }

    /// Clone of the sender for handing to the input task or other producers.
    pub fn sender(&self) -> Sender<AppEvent> {
        self.tx.clone()
    }

    pub fn publish(&self, event: AppEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&mut self, kind: EventKind, widget: WidgetId) {
        self.subscribers.entry(kind).or_default().insert(widget);
    }

    pub fn unsubscribe(&mut self, kind: EventKind, widget: WidgetId) {
        if let Some(subs) = self.subscribers.get_mut(&kind) {
            subs.remove(&widget);
        }
    }

    /// Drains and applies all pending events to `state` in order,
    /// returning the set of widgets whose subscribed event kinds fired
    /// this tick (for selective redraw).
    pub fn process(&self, state: &mut AppState) -> HashSet<WidgetId> {
        let mut dirty = HashSet::new();

        while let Ok(event) = self.rx.try_recv() {
            let kind = EventKind::from(&event);
            reduce(&event, state);

            if let Some(subs) = self.subscribers.get(&kind) {
                dirty.extend(subs.iter().copied());
            }
        }

        dirty
    }
}
