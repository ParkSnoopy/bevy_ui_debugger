//! The note record and where note lines go.

use std::{
    fmt::Write as _,
    fs::{File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
};

use bevy::{ecs::resource::Resource, math::Vec2};

use crate::inspect::UiTarget;

/// Every note line starts with this prefix so it can be filtered out of a noisy stderr.
pub const LOG_PREFIX: &str = "UI_DEBUG_NOTE";

/// File name used inside `log_dir`.
pub const LOG_FILE: &str = "ui-debug-notes.jsonl";

/// One submitted annotation together with the identity of the node it was written on.
#[derive(Clone, Debug)]
pub struct Note {
    /// 1-based sequence number within the session.
    pub seq: u32,
    /// Seconds of virtual time since the game started.
    pub time_secs: f64,
    /// The node that was annotated.
    pub target: UiTarget,
    /// Logical cursor position when the node was clicked.
    pub cursor: Vec2,
    /// Logical size of the primary window.
    pub window: Vec2,
    /// The annotation text.
    pub text: String,
}

impl Note {
    /// JSON object describing the note, one line, UTF-8 strings escaped.
    pub fn to_json(&self) -> String {
        let markers = self
            .target
            .markers
            .iter()
            .map(|m| json_str(m))
            .collect::<Vec<_>>()
            .join(",");
        let ancestry = self
            .target
            .ancestry
            .iter()
            .map(|a| json_str(a))
            .collect::<Vec<_>>()
            .join(",");
        let mut out = String::new();
        let _ = write!(
            out,
            "{{\"seq\":{},\"t\":{:.3},\"entity\":\"{}\",\"name\":{},\"markers\":[{}],\"text\":{},\
             \"rect\":[{:.1},{:.1},{:.1},{:.1}],\"z\":{},\"ancestry\":[{}],\"style\":{},\
             \"cursor\":[{:.1},{:.1}],\"window\":[{:.1},{:.1}],\"note\":{}}}",
            self.seq,
            self.time_secs,
            self.target.id,
            self.target.name.as_deref().map_or("null".to_string(), json_str),
            markers,
            self.target.text.as_deref().map_or("null".to_string(), json_str),
            self.target.rect.min.x,
            self.target.rect.min.y,
            self.target.rect.width(),
            self.target.rect.height(),
            self.target.z_index.map_or("null".to_string(), |z| z.to_string()),
            ancestry,
            json_str(&self.target.style),
            self.cursor.x,
            self.cursor.y,
            self.window.x,
            self.window.y,
            json_str(&self.text),
        );
        out
    }

    /// The full log line.
    pub fn to_line(&self) -> String {
        format!("{LOG_PREFIX} {}", self.to_json())
    }
}

/// Every note written during the session, newest last.
#[derive(Resource, Debug, Default)]
pub struct NoteHistory {
    /// All notes in submission order.
    pub notes: Vec<Note>,
}

/// Opening separator of the block printed at exit.
const BLOCK_START: &str = "--- UI_DEBUG_NOTES ---";

/// Closing separator of the block printed at exit.
const BLOCK_END: &str = "--- END UI_DEBUG_NOTES ---";

/// Appends note lines to a JSONL file when configured and keeps them in memory. The buffered
/// lines are printed to stderr as one block when the game ends, so they are not lost in the
/// engine's own log output.
#[derive(Resource)]
pub(crate) struct NoteSink {
    file: Option<File>,
    pending: Vec<String>,
}

impl NoteSink {
    /// Opens `<dir>/ui-debug-notes.jsonl` for appending, or memory-only when `dir` is `None`.
    pub(crate) fn open(dir: Option<PathBuf>) -> Self {
        let file = dir.and_then(|dir| open_log(&dir));
        Self {
            file,
            pending: Vec::new(),
        }
    }

    pub(crate) fn write(&mut self, line: &str) {
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
        self.pending.push(line.to_owned());
    }

    /// Prints everything buffered so far as one block and empties the buffer.
    pub(crate) fn print_all(&mut self) {
        if let Some(block) = format_block(&self.pending) {
            let _ = writeln!(std::io::stderr().lock(), "{block}");
        }
        self.pending.clear();
    }
}

impl Drop for NoteSink {
    fn drop(&mut self) {
        self.print_all();
    }
}

/// The exit block: a blank line, the opening separator, one line per note, the closing
/// separator and a blank line. `None` when there is nothing to print.
fn format_block(lines: &[String]) -> Option<String> {
    if lines.is_empty() {
        return None;
    }
    Some(format!(
        "\n{BLOCK_START}\n{}\n{BLOCK_END}\n",
        lines.join("\n")
    ))
}

fn open_log(dir: &Path) -> Option<File> {
    std::fs::create_dir_all(dir).ok()?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(LOG_FILE))
        .ok()
}

/// JSON string literal with the minimal escapes JSON requires.
pub(crate) fn json_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", control as u32);
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_is_framed_and_ordered() {
        let lines = vec!["UI_DEBUG_NOTE {a}".to_owned(), "UI_DEBUG_NOTE {b}".to_owned()];
        assert_eq!(
            format_block(&lines).as_deref(),
            Some("\n--- UI_DEBUG_NOTES ---\nUI_DEBUG_NOTE {a}\nUI_DEBUG_NOTE {b}\n--- END UI_DEBUG_NOTES ---\n")
        );
    }

    #[test]
    fn empty_buffer_prints_nothing() {
        assert_eq!(format_block(&[]), None);
    }

    #[test]
    fn print_all_drains_the_buffer() {
        let mut sink = NoteSink::open(None);
        sink.write("UI_DEBUG_NOTE {a}");
        sink.print_all();
        assert_eq!(sink.pending.len(), 0);
    }
}
