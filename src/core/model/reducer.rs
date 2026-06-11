// model/reducer.rs

use crate::core::controller::events::{
    AppEvent, AudioEvent, InputEvent, LibraryEvent, ScrollbackEvent,
};
use crate::core::model::state::AppState;
use crate::core::model::view_state::AppAction;

/// Single point of state mutation. Audio commands are dispatched to
/// the worker thread via a non-blocking channel send as part of the
/// audio reducer; library/scrollback/input reduction is pure
/// in-memory state mutation with no I/O.
pub fn reduce(event: &AppEvent, state: &mut AppState) -> Option<AppAction> {
    match event {
        AppEvent::Input(e) => reduce_input(e, state),
        AppEvent::Library(e) => {
            reduce_library(e, state);
            None
        }
        AppEvent::Audio(e) => {
            reduce_audio(e, state);
            None
        }
        AppEvent::Scrollback(e) => {
            reduce_scrollback(e, state);
            None
        }
    }
}

fn reduce_input(event: &InputEvent, state: &mut AppState) -> Option<AppAction> {
    match event {
        InputEvent::KeyPress(key) => {
            let track_count = state.tracks.len();
            let fs_entry_count = state.fs_entries.len();
            let action = state
                .view_state
                .handle_key(*key, track_count, fs_entry_count);

            match action {
                AppAction::Activate => Some(state.resolve_activation()),
                other => Some(other),
            }
        }
        InputEvent::Resize(w, h) => {
            state.view_state.resize(*w, *h);
            None
        }
        InputEvent::Mouse(_) | InputEvent::Paste(_) => None,
    }
}

fn reduce_library(event: &LibraryEvent, state: &mut AppState) {
    match event {
        LibraryEvent::TrackAdded(id) => state.library_cache.invalidate(id),
        LibraryEvent::TrackRemoved(id) => state.library_cache.remove(id),
        LibraryEvent::TrackUpdated(id) => state.library_cache.invalidate(id),
        LibraryEvent::PlaylistCreated(id)
        | LibraryEvent::PlaylistUpdated(id)
        | LibraryEvent::PlaylistDeleted(id) => state.library_cache.invalidate_playlist(id),
    }
}

fn reduce_audio(event: &AudioEvent, state: &mut AppState) {
    match event {
        AudioEvent::Play => state.audio.play(),
        AudioEvent::Pause => state.audio.pause(),
        AudioEvent::Stop => state.audio.stop(),
        AudioEvent::Seek(pos_ms) => state.audio.seek(*pos_ms),
        AudioEvent::Volume(level) => state.audio.set_volume(*level),
        AudioEvent::TrackChanged(id) => state.audio.load_track(id),
    }
}

fn reduce_scrollback(event: &ScrollbackEvent, state: &mut AppState) {
    match event {
        ScrollbackEvent::AddLine(line) => state.scrollback.push_line(line.clone()),
        ScrollbackEvent::Clear => state.scrollback.clear(),
        ScrollbackEvent::ScrollUp(n) => state.scrollback.scroll_up(*n),
        ScrollbackEvent::ScrollDown(n) => state.scrollback.scroll_down(*n),
    }
}
