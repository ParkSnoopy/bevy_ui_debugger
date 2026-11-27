# Usage

## Opting in

The overlay should never ship enabled by default.  
The crate reads no command-line arguments and no environment variables, so the host decides when to add the plugin and passes every option as a plugin field:

```rust
if args.ui_debug {
    app.add_plugins(UiDebuggerPlugin {
        verbose: args.verbose,
        ..Default::default()
    });
}
```

`args` is the host's own parsed command line, a config file, or whatever else the host uses.

### Verbose notes

Notes are short by default.  
Set `UiDebuggerPlugin::verbose` to `true` for the full detail.  
The host chooses how to expose it, for example as a `-v`/`--verbose` flag.

## Plugin settings

`UiDebuggerPlugin` is a plain struct; use struct-update syntax to change fields.

| Field | Default | Meaning |
| --- | --- | --- |
| `toggle_key` | `KeyCode::F10` | Shows or hides the whole overlay. |
| `log_dir` | `None` | Directory that receives `ui-debug-notes.jsonl`. Created if missing. Notes are also printed to stderr when the game ends. |
| `verbose` | `false` | Adds `seq`, `t`, `rect`, `style`, `cursor` and `window` to every note. |
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

Bevy's built-in font has no CJK glyphs.  
If the host UI shows such text, pass the host's font so the tooltip, the editor header and the typed note render correctly.

### Button placement

Pick a spot the host UI does not use.  
Check each screen of the host, since a corner that is free on one screen may hold a label on another.  
`Val::Auto` leaves an edge unconstrained, so setting `left` and `top` pins the button to the top-left, setting `right` and `bottom` to the bottom-right, and so on.

### Excluding decorative nodes

Add `bevy_ui_debugger::IgnoreUiDebugPick` to decorative nodes, such as a full-screen vignette, to let the debugger select the nodes behind them.
This marker does not affect rendering or Bevy gameplay picking, and does not exclude descendants.

### Deferring host input

The debugger resets gameplay button state during `InputSystems` while the
inspector or note editor owns input. Schedule gameplay button readers after
`InputSystems`. Hosts that buffer actions or consume raw events can also read
`Option<Res<UiDebuggerInput>>` after that set and discard gameplay input when its
value is `true`. The focused note editor still receives raw keyboard events.

## Workflow

1. Start the host with whatever switch it uses to add the plugin.
2. Click the `UI` button. It turns solid while inspect mode is on.
3. Hover nodes; the tooltip shows `#id "name" Marker "text" WxH @ (x,y)`.
4. Click a node. The editor opens below it, or above when there is no room below.
5. Type the note and press `Enter`. An empty note is discarded. `Esc` cancels.
6. A numbered pin appears left of the node. Pins follow the node and disappear when the node is despawned.
7. Click the `UI` button again, or press `Esc` with no editor open, to leave inspect mode.

## Reading notes back

Notes are written in two places:

- **stderr**, once, when the game ends. The engine's own output is verbose, so notes are held in memory and printed together as one block, one line each, prefixed with `UI_DEBUG_NOTE `:
  ```text
  
  --- UI_DEBUG_NOTES ---
  UI_DEBUG_NOTE {...}
  UI_DEBUG_NOTE {...}
  --- END UI_DEBUG_NOTES ---
  
  ```
  Nothing is printed when no note was saved.
- **`<log_dir>/ui-debug-notes.jsonl`**, the same lines, appended as each note is saved so they survive a crash. The file is opened in append mode, so several sessions accumulate. Run with `verbose` to get the `seq` and `t` fields that tell them apart.

Inside the process every note is also kept in the `NoteHistory` resource (`notes: Vec<Note>`), newest last, for tests or custom export.

Field meanings are in [note-format.md](note-format.md).
