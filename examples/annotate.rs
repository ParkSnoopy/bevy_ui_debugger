//! A tiny host game that shows the debugger in action.
//!
//! Interactive: `cargo run --example annotate`
//! Scripted session with screenshots and a log file: `cargo run --example annotate -- --script ./shots`

use std::path::PathBuf;

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyCode, KeyboardInput, NativeKeyCode},
        mouse::MouseButtonInput,
    },
    prelude::*,
    render::view::screenshot::{save_to_disk, Screenshot},
    ui_widgets::Button,
    window::{CursorMoved, PrimaryWindow, WindowEvent},
};
use bevy_ui_debugger::{NoteHistory, UiDebuggerPlugin};

#[derive(Component)]
struct MenuButton;

#[derive(Resource)]
struct Script {
    frame: u32,
    typed: usize,
    dir: PathBuf,
}

/// Centre of the toggle button in the bottom-right corner of the 1280x720 window.
const PENCIL: Vec2 = Vec2::new(1242.0, 688.0);
const BTN_LOAD: Vec2 = Vec2::new(130.0, 150.0);
const BTN_SETTINGS: Vec2 = Vec2::new(130.0, 206.0);
const HUD_TEXT: Vec2 = Vec2::new(790.0, 356.0);
const NOTE_A: &str = "Load button: too small, move it up";
const NOTE_B: &str = "Make this bigger";

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let script = args
        .iter()
        .position(|arg| arg == "--script")
        .map(|index| PathBuf::from(args.get(index + 1).map_or("./shots", String::as_str)));
    let window = Window {
        title: "bevy_ui_debugger demo".into(),
        resolution: (1280, 720).into(),
        ..default()
    };
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(window),
            ..default()
        }),
    );
    // A real game gates this on its own command-line flag or setting.
    app.add_plugins(UiDebuggerPlugin {
        log_dir: script.clone(),
        verbose: args.iter().any(|arg| arg == "--verbose"),
        ..Default::default()
    });
    if let Some(dir) = script {
        std::fs::create_dir_all(&dir)?;
        app.insert_resource(Script {
            frame: 0,
            typed: 0,
            dir,
        });
        app.add_systems(Update, run_script);
    }
    app.add_systems(Startup, spawn_ui).run();
    Ok(())
}

fn spawn_ui(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Name::new("top_bar"),
                Node {
                    height: Val::Px(56.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    column_gap: Val::Px(16.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.11, 0.14)),
            ))
            .with_children(|bar| {
                bar.spawn((
                    Text::new("DEMO GAME"),
                    TextFont {
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                ));
                bar.spawn((
                    Text::new("depth 3"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.6, 0.64, 0.7)),
                ));
            });
            root.spawn(Node { flex_grow: 1.0, ..default() })
                .with_children(|body| {
                    body.spawn((
                        Name::new("side_panel"),
                        Node {
                            width: Val::Px(260.0),
                            padding: UiRect::all(Val::Px(16.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(12.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.11, 0.12, 0.15)),
                    ))
                    .with_children(|panel| {
                        for (name, caption) in [
                            ("btn_new_run", "New run"),
                            ("btn_load", "Load"),
                            ("", "Settings"),
                        ] {
                            let mut button = panel.spawn((
                                Node {
                                    height: Val::Px(44.0),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                Button,
                                MenuButton,
                                BackgroundColor(Color::srgb(0.2, 0.22, 0.28)),
                            ));
                            if !name.is_empty() {
                                button.insert(Name::new(name));
                            }
                            button.with_child((
                                Text::new(caption),
                                TextFont {
                                    font_size: FontSize::Px(20.0),
                                    ..default()
                                },
                            ));
                        }
                    });
                    body.spawn((
                        Name::new("hud_depth"),
                        Node {
                            flex_grow: 1.0,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.16, 0.18, 0.22)),
                    ))
                    .with_child((
                        Text::new("depth 3 / courage 12"),
                        TextFont {
                            font_size: FontSize::Px(28.0),
                            ..default()
                        },
                    ));
                });
            root.spawn((
                Name::new("bottom_bar"),
                Node {
                    height: Val::Px(64.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.11, 0.14)),
            ))
            .with_child((
                Text::new("Esc menu / F1 help"),
                TextFont {
                    font_size: FontSize::Px(16.0),
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.64, 0.7)),
            ));
        });
}

fn run_script(world: &mut World) {
    if !world.contains_resource::<Script>() {
        return;
    }
    let frame = {
        let mut script = world.resource_mut::<Script>();
        script.frame += 1;
        script.frame
    };
    match frame {
        10 => move_cursor(world, PENCIL),
        14 | 34 | 104 => click(world),
        20 => move_cursor(world, BTN_LOAD),
        30 => shot(world, "01_hover"),
        40 => shot(world, "02_editor"),
        44..=78 => type_text(world, NOTE_A),
        82 => shot(world, "03_typed"),
        86 | 134 => press(world, KeyCode::Enter, &Key::Enter),
        96 => move_cursor(world, HUD_TEXT),
        112..=127 => type_text(world, NOTE_B),
        140 => move_cursor(world, BTN_SETTINGS),
        150 => shot(world, "04_pins"),
        156 => finish(world),
        _ => {}
    }
}

fn primary_window(world: &mut World) -> Option<Entity> {
    world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .ok()
}

fn move_cursor(world: &mut World, position: Vec2) {
    let Some(window) = primary_window(world) else {
        return;
    };
    let scale = world
        .get_entity(window)
        .ok()
        .and_then(|entity| entity.get::<Window>())
        .map_or(1.0, Window::scale_factor);
    if let Some(mut window) = world.get_mut::<Window>(window) {
        window.set_physical_cursor_position(Some((position * scale).as_dvec2()));
    }
    world.write_message(CursorMoved {
        window,
        position,
        delta: None,
    });
    world.write_message(WindowEvent::CursorMoved(CursorMoved {
        window,
        position,
        delta: None,
    }));
}

fn click(world: &mut World) {
    let Some(window) = primary_window(world) else {
        return;
    };
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let input = MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        };
        world.write_message(input);
        world.write_message(WindowEvent::from(input));
    }
}

/// Sends a press and its release, like a real key tap.
fn tap(world: &mut World, key_code: KeyCode, logical_key: &Key, text: Option<&str>) {
    let Some(window) = primary_window(world) else {
        return;
    };
    for state in [ButtonState::Pressed, ButtonState::Released] {
        let input = KeyboardInput {
            key_code,
            logical_key: logical_key.clone(),
            state,
            text: (state == ButtonState::Pressed)
                .then(|| text.map(Into::into))
                .flatten(),
            repeat: false,
            window,
        };
        world.write_message(input.clone());
        world.write_message(WindowEvent::KeyboardInput(input));
    }
}

fn press(world: &mut World, key_code: KeyCode, logical_key: &Key) {
    tap(world, key_code, logical_key, None);
}

fn type_text(world: &mut World, text: &str) {
    let Some(ch) = text.chars().nth(world.resource::<Script>().typed) else {
        world.resource_mut::<Script>().typed = 0;
        return;
    };
    world.resource_mut::<Script>().typed += 1;
    let typed = ch.to_string();
    tap(
        world,
        KeyCode::Unidentified(NativeKeyCode::Unidentified),
        &Key::Character(typed.as_str().into()),
        Some(&typed),
    );
}

fn shot(world: &mut World, name: &str) {
    let path = world.resource::<Script>().dir.join(format!("{name}.png"));
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn finish(world: &mut World) {
    let count = world.resource::<NoteHistory>().notes.len();
    if count == 2 {
        info!("script ok: {count} notes");
        world.write_message(AppExit::Success);
    } else {
        error!("script failed: expected 2 notes, got {count}");
        world.write_message(AppExit::error());
    }
}
