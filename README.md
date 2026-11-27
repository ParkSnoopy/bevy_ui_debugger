# bevy_ui_debugger

An opt-in overlay for [Bevy](https://bevy.org) that lets you point at a UI node in a running game,  
type a note about it, and get back one log line that identifies the node exactly.

It exists to close the gap between "this button looks wrong" and the entity, component and style that produce it.  
Review feedback written against the running UI can be pasted straight into a bug report, an issue or an agent prompt.

## What it does

- A small `UI` button toggles **inspect mode**.  
- In inspect mode the topmost UI node under the cursor is highlighted,  
  and a tooltip shows its identity.
- Clicking a node opens a one-line editor.  
  `Enter` saves, `Esc` cancels.  
- Each saved note is a `UI_DEBUG_NOTE {...}` JSON line.  
  Lines are kept in memory and printed to stderr as one block between `---` separators when the game ends,  
  and appended to a JSONL file as they are saved when a log directory is configured.
- A numbered pin stays next to every annotated node.  
- While inspecting, clicks and keys are kept away from the host game.

## Quick start

```toml
[dependencies]
bevy_ui_debugger = { git = "https://github.com/ParkSnoopy/bevy_ui_debugger", version = "0.1" }
```

```rust
use bevy::prelude::*;
use bevy_ui_debugger::UiDebuggerPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(UiDebuggerPlugin::default());
    app.run();
}
```

The crate reads no command-line flags or environment variables.  
Gate the plugin on the host's own switch and pass options as plugin fields.  
`F10` shows or hides the whole overlay.  
Set `verbose: true` for the full note detail (geometry, style, cursor, window).

Try it without writing any code:

```sh
cargo run --example annotate
# scripted session that saves screenshots and a log file
cargo run --example annotate -- --script ./shots
```

## Bevy support

| bevy | bevy_ui_debugger |
| --- | --- |
| 0.20 | 0.1 |

## Requirements

- A primary window. Hit testing and note coordinates use the primary window only.  
- Systems of the host game that read input must run `.after(InputSystems)`;  
  see [docs/input-handling.md](docs/input-handling.md).

## Documentation

| Document | Contents |
| --- | --- |
| [docs/usage.md](docs/usage.md) | Opt-in, plugin settings, fonts, button placement, reading notes back |
| [docs/note-format.md](docs/note-format.md) | The exact shape of a note line |
| [docs/input-handling.md](docs/input-handling.md) | How input is isolated and what the host must do |
| [docs/architecture.md](docs/architecture.md) | Module layout and the per-frame flow |

## Cargo features

| Feature | Default | Effect |
| --- | --- | --- |
| `component-names` | on | Enables Bevy's `debug` feature so component type names are available at runtime and notes can list marker components. Without it `markers` is empty. |

## License

LGPL-3.0-or-later. See [LICENSE](LICENSE).
