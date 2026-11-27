# Input handling

The overlay sits on top of a running game, so it has to take clicks and keys without the game  
reacting to them as well.

## What the overlay takes

- the toggle key (`F10` by default)  
- a left click on the `UI` button, whether or not inspect mode is on  
- `Esc` while inspect mode is on and no editor is open  
- every mouse button and key while inspect mode is on

Modifier keys that are held stay held, so a held `Shift` or `Ctrl` does not appear to be  
released to the game.

## How it works

`suppress_game_input` runs in `PreUpdate` inside Bevy's `InputSystems` set, after  
`keyboard_input_system` and `mouse_button_input_system` have filled `ButtonInput<KeyCode>`,  
`ButtonInput<Key>` and `ButtonInput<MouseButton>`, and before `InputFocusSystems::Dispatch`.  
When input should be taken it clears those three resources, then re-presses held modifiers.

The decision to swallow comes from two earlier systems, `read_hotkey` and `capture_click`. They  
read the raw `KeyboardInput` and `MouseButtonInput` messages, so they see events that the  
clearing step removes from the `ButtonInput` state.

## What the host must do

Any host system that reads `ButtonInput` has to run after the clearing step:

```rust
app.add_systems(PreUpdate, my_input_system.after(InputSystems));
// or in Update, which already runs after PreUpdate
app.add_systems(Update, my_input_system);
```

A system in `PreUpdate` that is not ordered after `InputSystems` may run first and see the  
click anyway. Systems in `Update` are always late enough.

Input read from raw messages (`MessageReader<KeyboardInput>`, `MessageReader<MouseButtonInput>`)  
is not filtered, and neither are gamepad and touch input.

## Picking

While inspect mode is on, the overlay root blocks lower picking targets, so Bevy's UI picking  
does not hover or press nodes underneath it. The root ignores picking when inspect mode is off.  
The `UI` button always blocks picking beneath itself.

## Focus

The editor takes `InputFocus` when it opens and releases it when it closes. Text typed into  
the editor therefore goes to the editor and not to other focused widgets.
