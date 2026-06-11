use crossterm::event::{KeyEvent, MouseEvent};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Input,
    Library,
    Audio,
    Scrollback,
}

#[derive(Clone, Debug)]
pub enum AppEvent {
    Input(InputEvent),
    Library(LibraryEvent),
    Audio(AudioEvent),
    Scrollback(ScrollbackEvent),
}

impl From<&AppEvent> for EventKind {
    fn from(event: &AppEvent) -> Self {
        match event {
            AppEvent::Input(_) => EventKind::Input,
            AppEvent::Library(_) => EventKind::Library,
            AppEvent::Audio(_) => EventKind::Audio,
            AppEvent::Scrollback(_) => EventKind::Scrollback,
        }
    }
}

#[derive(Clone, Debug)]
pub enum InputEvent {
    KeyPress(KeyEvent),
    Resize(u16, u16),
    Mouse(MouseEvent),
    Paste(String),
}

#[derive(Clone, Debug)]
pub enum LibraryEvent {
    TrackAdded(String),
    TrackRemoved(String),
    TrackUpdated(String),
    PlaylistCreated(String),
    PlaylistUpdated(String),
    PlaylistDeleted(String),
}

#[derive(Clone, Debug)]
pub enum AudioEvent {
    Play,
    Pause,
    Stop,
    Seek(u64),
    Volume(f32),
    TrackChanged(String),
}

#[derive(Clone, Debug)]
pub enum ScrollbackEvent {
    AddLine(String),
    Clear,
    ScrollUp(u16),
    ScrollDown(u16),
}
