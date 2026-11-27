# Note format

A note line is the literal prefix `UI_DEBUG_NOTE`, one space, then a single-line JSON object.  
Strings are UTF-8 with the minimal JSON escapes (`"`, `\`, `\n`, `\r`, `\t`, other control  
characters as `\u00XX`).

```text
UI_DEBUG_NOTE {"entity":"123v4294967296","name":"btn_load","markers":["my_game::ui::MenuButton"],"text":"Load","z":null,"ancestry":["Node#7","side_panel"],"note":"too small, move it up"}
```

The short form above is the default. With `UiDebuggerPlugin::verbose` the object also carries `seq`, `t`,  
`rect`, `style`, `cursor` and `window`, in this order:

```text
UI_DEBUG_NOTE {"seq":1,"t":12.480,"entity":"123v4294967296","name":"btn_load","markers":["my_game::ui::MenuButton"],"text":"Load","rect":[16.0,150.0,228.0,44.0],"z":null,"ancestry":["Node#7","side_panel"],"style":"display=Flex ...","cursor":[130.0,152.0],"window":[1280.0,720.0],"note":"too small, move it up"}
```

## Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `seq` (verbose) | integer | 1-based sequence number within the session. |
| `t` (verbose) | number | Seconds of virtual time since the game started, three decimals. |
| `entity` | string | `<index>v<generation>` of the annotated entity. Unique for one run of the game, not across runs. |
| `name` | string or null | The `Name` component, when present. |
| `markers` | string array | Type paths of the node's components that are not part of the engine, for example `my_game::ui::MenuButton`. Empty when Bevy's `debug` feature is off. Anything whose path starts with `bevy::` or `bevy_` is skipped. |
| `text` | string or null | Text of the node and of its descendants, joined with ` / `. Looks at most 3 levels down, collects at most 10 pieces, truncated to 120 characters. |
| `rect` (verbose) | `[x, y, w, h]` | Bounding box in logical pixels, top-left origin, the same space as `Window::cursor_position`. |
| `z` | integer or null | `GlobalZIndex` when present, otherwise `ZIndex`. |
| `ancestry` | string array | Labels from the root to the parent of the node, at most 12. A label is the `Name`, else the first marker's short name, else `Node#<index>`. |
| `style` (verbose) | string | One-line dump of the `Node` style: display, position, sizes, padding, margin, border, flex settings, alignment, gaps, overflow and insets. |
| `cursor` (verbose) | `[x, y]` | Logical cursor position at the click. |
| `window` (verbose) | `[w, h]` | Logical size of the primary window. |
| `note` | string | The text the reviewer typed, trimmed. |

## Using a note

To find the node in code, search for the marker type or the `Name`. When neither exists the  
`ancestry` and `text` fields usually locate it. To reproduce a layout problem, enable `verbose`:  
`rect`, `window` and `style` give the observed geometry, so compare them with what the code  
intends.

## Stability

The set of fields is not versioned. New fields may be added at the end of the object; existing  
fields keep their names and meaning. Parse by key, not by position.
