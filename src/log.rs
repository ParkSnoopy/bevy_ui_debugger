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

/// Writes note lines to stderr and, when configured, to a JSONL file.
#[derive(Resource)]
pub(crate) struct NoteSink {
    file: Option<File>,
}

impl NoteSink {
    /// Opens `<dir>/ui-debug-notes.jsonl` for appending, or stderr-only when `dir` is `None`.
    pub(crate) fn open(dir: Option<PathBuf>) -> Self {
        let file = dir.and_then(|dir| open_log(&dir));
        Self { file }
    }

    pub(crate) fn write(&mut self, line: &str) {
        eprintln!("{line}");
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
        }
    }
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
