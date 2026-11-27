# Usage

## Opting in

The overlay should never ship enabled by default. `bevy_ui_debugger::is_requested()` returns  
true when either of these is present:

- the command-line flag `--ui-debug` (`bevy_ui_debugger::FLAG`)  
- the environment variable `BEVY_UI_DEBUG` set to anything other than empty or `0`  
  (`bevy_ui_debugger::ENV`)

```rust
if bevy_ui_debugger::is_requested() {
    app.add_plugins(UiDebuggerPlugin::default());
}
```

If the host parses its command line strictly (for example with `clap` derive), declare  
`--ui-debug` in the host's own parser and use `is_requested()` only as a fallback for the  
environment variable. The function scans `std::env::args_os` and does not consume anything.

## Plugin settings

`UiDebuggerPlugin` is a plain struct; use struct-update syntax to change fields.

| Field | Default | Meaning |
| --- | --- | --- |
| `toggle_key` | `KeyCode::F10` | Shows or hides the whole overlay. |
| `log_dir` | `None` | Directory that receives `ui-debug-notes.jsonl`. Created if missing. Notes always go to stderr as well. |
| `font` | `None` | Font for all overlay text. Without it Bevy's default font is used when one is compiled in. |
| `button_position` | bottom-right, 12 px inset | `UiRect` of absolute offsets for the toggle button. |

```rust
app.add_plugins(UiDebuggerPlugin {
    log_dir: Some("./notes".into()),
    font: Some(asset_server.load("fonts/MyFont.ttf")),
    button_position: UiRect::new(Val::Px(12.0), Val::Auto, Val::Percent(50.0), Val::Auto),
    ..Default::default()
});
```

### Fonts and non-Latin text

Bevy's built-in font has no CJK glyphs. If the host UI shows such text, pass the host's font so  
the tooltip, the editor header and the typed note render correctly.

### Button placement

Pick a spot the host UI does not use. Check each screen of the host, since a corner that is free  
on one screen may hold a label on another. `Val::Auto` leaves an edge unconstrained, so setting  
`left` and `top` pins the button to the top-left, setting `right` and `bottom` to the  
bottom-right, and so on.

## Workflow

1. Start the host with `--ui-debug`.  
2. Click the `UI` button. It turns solid while inspect mode is on.  
3. Hover nodes; the tooltip shows `#id "name" Marker "text" WxH @ (x,y)`.  
4. Click a node. The editor opens below it, or above when there is no room below.  
5. Type the note and press `Enter`. An empty note is discarded. `Esc` cancels.  
6. A numbered pin appears left of the node. Pins follow the node and disappear when the node  
   is despawned.
7. Click the `UI` button again, or press `Esc` with no editor open, to leave inspect mode.

## Reading notes back

Notes are written in two places:

- **stderr**, one line each, prefixed with `UI_DEBUG_NOTE `. Filter a noisy log with  
  `grep '^UI_DEBUG_NOTE '`.
- **`<log_dir>/ui-debug-notes.jsonl`**, the same lines. The file is opened in append mode, so  
  several sessions accumulate. Use the `seq` and `t` fields to tell them apart.

Inside the process every note is also kept in the `NoteHistory` resource (`notes: Vec<Note>`),  
newest last, for tests or custom export.

Field meanings are in [note-format.md](note-format.md).
