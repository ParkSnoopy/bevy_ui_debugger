//! In-game UI annotation overlay for Bevy.
//!
//! Add [`UiDebuggerPlugin`] to the app, typically only when the host's own switch asks for
//! it. The crate reads neither command-line arguments nor environment variables; every
//! option is a field of the plugin.
//!
//! Game input is muted while inspecting; game systems that read `ButtonInput` must run
//! `.after(InputSystems)` for that to hold.
//!
//! A small button (bottom-right unless `button_position` says otherwise) toggles inspect mode. In inspect mode the topmost
//! UI node under the cursor is highlighted, clicking it opens a text field, and submitting
//! the text with Enter records one `UI_DEBUG_NOTE {...}` line that identifies the node: entity
//! id, name, component markers, text, on-screen rect, ancestry and style. Lines are printed
//! to stderr as one `---` delimited block when the game ends, and appended to a JSONL file
//! as they are saved when `log_dir` is set.

#![expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system parameters such as `Res` and `ResMut` can only be taken by value"
)]

mod inspect;
mod log;
mod overlay;

pub use inspect::{IgnoreUiDebugPick, UiTarget};
pub use log::{LOG_PREFIX, Note, NoteHistory};
pub use overlay::{UiDebuggerInput, UiDebuggerPlugin};
