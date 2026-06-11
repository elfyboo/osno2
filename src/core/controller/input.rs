use crossbeam_channel::Sender;
use crossterm::event::{Event as CtEvent, EventStream, KeyEventKind};
use futures_util::StreamExt;

use crate::core::controller::events::{AppEvent, InputEvent};

pub async fn poll_loop(tx: Sender<AppEvent>) {
    let mut stream = EventStream::new();

    while let Some(result) = stream.next().await {
        let ct_event = match result {
            Ok(event) => event,
            Err(_) => break,
        };

        let Some(input_event) = translate(ct_event) else {
            continue;
        };

        if tx.send(AppEvent::Input(input_event)).is_err() {
            break;
        }
    }
}

fn translate(event: CtEvent) -> Option<InputEvent> {
    match event {
        CtEvent::Key(key) if key.kind == KeyEventKind::Press => Some(InputEvent::KeyPress(key)),
        CtEvent::Resize(w, h) => Some(InputEvent::Resize(w, h)),
        CtEvent::Mouse(mouse) => Some(InputEvent::Mouse(mouse)),
        CtEvent::Paste(text) => Some(InputEvent::Paste(text)),
        _ => None,
    }
}
