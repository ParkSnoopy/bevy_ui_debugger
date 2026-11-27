# Architecture

## Modules

| File | Role |
| --- | --- |
| `src/lib.rs` | Public re-exports and the crate-level overview. |
| `src/log.rs` | `Note`, `NoteHistory`, the in-memory buffer printed at exit, the JSONL sink, JSON string escaping. |
| `src/inspect.rs` | Hit testing over `UiStack` and identity extraction (`UiTarget`). |
| `src/overlay.rs` | The plugin: overlay entities, input isolation, editor, pins, tooltip. |
| `examples/annotate.rs` | Stand-alone demo game with an optional scripted session. |

Public API: `UiDebuggerPlugin`, `UiTarget`, `Note`, `NoteHistory`, `LOG_PREFIX`.

## Overlay entities

`spawn_overlay` creates one absolutely positioned full-window root with `GlobalZIndex(i32::MAX)`.  
Its children are the highlight rectangle, the pins container, the tooltip and the toggle button.  
The editor panel is a separate root with its own `GlobalZIndex(i32::MAX)`, spawned when a node is selected and despawned when the selection ends.

Every overlay node carries the private `OverlayNode` marker.  
Hit testing skips such nodes, which is how the overlay avoids selecting itself, and clicks on the button or editor are recognised by checking whether the hit node is inside their subtree.

## Per-frame flow

`PreUpdate`

1. `read_hotkey`, `capture_click` (chained): read raw input messages, record a toggle request, the click position and whether the click or key must be swallowed.
2. `suppress_game_input`, in `InputSystems`: clears `ButtonInput` state when needed. See [input-handling.md](input-handling.md).

`Update`, chained in this order

1. `refresh_mode`: applies a toggle request, updates visibility, picking and button colours.
2. `update_hover`: finds the topmost node under the cursor in inspect mode, moves the highlight.
3. `handle_click`: a click on the button toggles inspect mode; a click inside the editor is ignored; any other click selects the topmost node.
4. `describe`: exclusive system. Builds a `UiTarget` for the hovered and the selected node only when they changed, then applies a pending editor action (save or cancel).
5. `sync_editor`: opens or closes the editor to match the selection.
6. `update_pins`: spawns the pin for a just-saved note and repositions the existing ones.
7. `update_tooltip`: shows the hovered node's summary next to the cursor.

## Hit testing

`topmost_node_at` walks `UiStack::uinodes` from the top.  
A node is accepted when it is not hidden (`InheritedVisibility`), contains the cursor in physical pixels (`cursor * scale`), and passes its `CalculatedClip`.  
The result rectangle is converted back to logical pixels.

## Identity extraction

`describe` reads `Node`, `ComputedNode` and `UiGlobalTransform`, plus `Name`, `Text`, `TextSpan`, `ZIndex`, `GlobalZIndex`, `ChildOf` and `Children`.  
Component type paths come from the `AppTypeRegistry`, falling back to the component info name.  
That fallback is an angle-bracket placeholder unless Bevy's `debug` feature is on, which is why the crate enables it through its default `component-names` feature.

## Design limits

- Only the primary window is handled.
- Only Bevy UI (`Node`) entities are inspectable; world-space sprites and meshes are not.
- The overlay depends on `bevy_ui_widgets` for the text input. It adds `UiWidgetsPlugins` unless a `TextInputPlugin` is already present.
- Note lines are written synchronously with `eprintln!` and an append-mode file flush per note. Notes are rare, so there is no buffering.
