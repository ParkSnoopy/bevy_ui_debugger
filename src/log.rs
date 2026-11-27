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

/// One submitted annotation about the game or a specific UI node.
#[derive(Clone, Debug)]
pub struct Note {
    /// 1-based sequence number within the session.
    pub seq: u32,
    /// Seconds of virtual time since the game started.
    pub time_secs: f64,
    /// The node that was annotated, or `None` for a whole-game note.
    pub target: Option<UiTarget>,
    /// Logical cursor position when the editor was opened.
    pub cursor: Vec2,
    /// Logical size of the primary window.
    pub window: Vec2,
    /// The annotation text.
    pub text: String,
}

impl Note {
    /// JSON object describing the note, one line, UTF-8 strings escaped.
    ///
    /// Whole-game notes use `scope: "game"` in place of node fields. `verbose`
    /// adds session and click geometry, plus `rect` and `style` for node notes.
    pub fn to_json(&self, verbose: bool) -> String {
        let strings = |items: &[String]| {
            items
                .iter()
                .map(|item| json_str(item))
                .collect::<Vec<_>>()
                .join(",")
        };
        let mut out = String::from("{");
        if verbose {
            let _ = write!(out, "\"seq\":{},\"t\":{:.3},", self.seq, self.time_secs);
        }
        if let Some(target) = &self.target {
            let _ = write!(
                out,
                "\"entity\":\"{}\",\"name\":{},\"markers\":[{}],\"text\":{},",
                target.id,
                target.name.as_deref().map_or("null".to_string(), json_str),
                strings(&target.markers),
                target.text.as_deref().map_or("null".to_string(), json_str),
            );
            if verbose {
                let _ = write!(
                    out,
                    "\"rect\":[{:.1},{:.1},{:.1},{:.1}],",
                    target.rect.min.x,
                    target.rect.min.y,
                    target.rect.width(),
                    target.rect.height(),
                );
            }
            let _ = write!(
                out,
                "\"z\":{},\"ancestry\":[{}],",
                target.z_index.map_or("null".to_string(), |z| z.to_string()),
                strings(&target.ancestry),
            );
            if verbose {
                let _ = write!(out, "\"style\":{},", json_str(&target.style));
            }
        } else {
            out.push_str("\"scope\":\"game\",");
        }
        if verbose {
            let _ = write!(
                out,
                "\"cursor\":[{:.1},{:.1}],\"window\":[{:.1},{:.1}],",
                self.cursor.x, self.cursor.y, self.window.x, self.window.y,
            );
        }
        let _ = write!(out, "\"note\":{}}}", json_str(&self.text));
        out
    }

    /// The full log line.
    pub fn to_line(&self, verbose: bool) -> String {
        format!("{LOG_PREFIX} {}", self.to_json(verbose))
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
        let lines = vec![
            "UI_DEBUG_NOTE {a}".to_owned(),
            "UI_DEBUG_NOTE {b}".to_owned(),
        ];
        assert_eq!(
            format_block(&lines).as_deref(),
            Some(
                "\n--- UI_DEBUG_NOTES ---\nUI_DEBUG_NOTE {a}\nUI_DEBUG_NOTE {b}\n--- END UI_DEBUG_NOTES ---\n"
            )
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

    fn sample_note() -> Note {
        Note {
            seq: 1,
            time_secs: 11.856,
            target: Some(UiTarget {
                entity: bevy::ecs::entity::Entity::PLACEHOLDER,
                id: "329v0".to_owned(),
                name: Some("top_bar".to_owned()),
                markers: Vec::new(),
                text: Some("DEMO GAME / depth 3".to_owned()),
                rect: bevy::math::Rect::new(0.0, 0.0, 1280.0, 56.0),
                z_index: Some(0),
                ancestry: vec!["Node#328".to_owned()],
                style: "display=Flex".to_owned(),
            }),
            cursor: Vec2::new(996.6, 36.1),
            window: Vec2::new(1280.0, 720.0),
            text: "test for heading".to_owned(),
        }
    }

    #[test]
    fn short_note_keeps_only_the_locating_fields() {
        assert_eq!(
            sample_note().to_line(false),
            "UI_DEBUG_NOTE {\"entity\":\"329v0\",\"name\":\"top_bar\",\"markers\":[],\
             \"text\":\"DEMO GAME / depth 3\",\"z\":0,\"ancestry\":[\"Node#328\"],\
             \"note\":\"test for heading\"}"
        );
    }

    #[test]
    fn verbose_note_adds_geometry_and_session_fields() {
        let line = sample_note().to_line(true);
        assert!(line.contains("\"seq\":1,\"t\":11.856,"));
        assert!(line.contains("\"rect\":[0.0,0.0,1280.0,56.0]"));
        assert!(line.contains("\"style\":\"display=Flex\""));
        assert!(line.contains("\"cursor\":[996.6,36.1],\"window\":[1280.0,720.0],"));
    }

    #[test]
    fn game_note_omits_ui_identity_and_geometry() {
        let note = Note {
            target: None,
            text: "Camera moves too fast".to_owned(),
            ..sample_note()
        };
        assert_eq!(
            note.to_line(false),
            "UI_DEBUG_NOTE {\"scope\":\"game\",\"note\":\"Camera moves too fast\"}"
        );
        assert_eq!(
            note.to_line(true),
            "UI_DEBUG_NOTE {\"seq\":1,\"t\":11.856,\"scope\":\"game\",\
             \"cursor\":[996.6,36.1],\"window\":[1280.0,720.0],\"note\":\"Camera moves too fast\"}"
        );
    }
}
