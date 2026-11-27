//! In-game UI annotation overlay for Bevy.
//!
//! Add [`UiDebuggerPlugin`] to the app, typically only when [`is_requested`] is true
//! (`--ui-debug` on the command line or `BEVY_UI_DEBUG=1`).
//!
//! Game input is muted while inspecting; game systems that read `ButtonInput` must run
//! `.after(InputSystems)` for that to hold.
//!
//! A small button (bottom-right unless `button_position` says otherwise) toggles inspect mode. In inspect mode the topmost
//! UI node under the cursor is highlighted, clicking it opens a text field, and submitting
//! the text with Enter writes one `UI_DEBUG_NOTE {...}` line to stderr (and to a JSONL file
//! when `log_dir` is set) that identifies the node: entity id, name, component markers,
//! text, on-screen rect, ancestry and style.

mod cli;
mod inspect;
mod log;
mod overlay;

pub use cli::{ENV, FLAG, is_requested};
pub use inspect::UiTarget;
pub use log::{LOG_PREFIX, Note, NoteHistory};
pub use overlay::UiDebuggerPlugin;
