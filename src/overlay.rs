//! The overlay: pencil toggle, hover highlight, tooltip, note editor and pins.

use std::path::PathBuf;

use bevy::{
    app::{App, Plugin, PreUpdate, Startup, Update},
    asset::Handle,
    color::Color,
    ecs::{
        entity::Entity,
        hierarchy::ChildOf,
        message::MessageReader,
        query::With,
        system::{Commands, Query, Res, ResMut},
        world::World,
    },
    input::{
        ButtonInput, ButtonState, InputSystems,
        keyboard::{Key, KeyCode, KeyboardInput, keyboard_input_system},
        mouse::{MouseButton, MouseButtonInput, mouse_button_input_system},
    },
    input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusSystems},
    picking::Pickable,
    prelude::*,
    text::{EditableText, Font, FontSize, FontSource, Justify, LineBreak, TextColor, TextCursorStyle, TextFont, TextLayout},
    time::Time,
    ui::{widget::Text, ComputedNode, UiGlobalTransform, UiStack},
    ui_widgets::{TextInput, TextInputPlugin, UiWidgetsPlugins},
    window::{PrimaryWindow, Window},
};

use crate::{
    inspect::{self, GameNodes, OverlayNode, UiTarget},
    log::{Note, NoteHistory, NoteSink},
};

const ACCENT: Color = Color::srgb(1.0, 0.72, 0.2);
const PANEL_BG: Color = Color::srgb(0.09, 0.10, 0.13);
const FIELD_BG: Color = Color::srgb(0.04, 0.04, 0.06);
const MUTED: Color = Color::srgb(0.62, 0.66, 0.73);
const EDITOR_WIDTH: f32 = 560.0;
const EDITOR_HEIGHT: f32 = 240.0;
const PIN_SIZE: f32 = 30.0;
/// Font size of all overlay text, in logical pixels.
const FONT_PX: f32 = 20.0;

#[derive(Component)]
struct OverlayRoot;
#[derive(Component)]
struct PencilButton;
#[derive(Component)]
struct PencilLabel;
#[derive(Component)]
struct HighlightRect;
#[derive(Component)]
struct Tooltip;
#[derive(Component)]
struct TooltipLabel;
#[derive(Component)]
struct PinsRoot;
#[derive(Component)]
struct EditorPanel;
#[derive(Component)]
struct EditorInput;

#[derive(Component)]
struct Pin {
    target: Entity,
}

#[derive(Resource)]
struct Config {
    toggle_key: KeyCode,
    font: Option<Handle<Font>>,
    button_position: UiRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorAction {
    Submit,
    Cancel,
}

/// Requests recorded in `PreUpdate` and consumed later in the same frame.
#[derive(Default)]
struct Pending {
    /// The toggle key was pressed.
    toggle: bool,
    /// This frame's input was meant for the overlay even though inspect mode is off.
    swallow: bool,
}

#[derive(Resource, Default)]
struct OverlayState {
    root: Option<Entity>,
    pencil: Option<Entity>,
    highlight: Option<Entity>,
    tooltip: Option<Entity>,
    pins_root: Option<Entity>,
    editor_panel: Option<Entity>,
    editor_input: Option<Entity>,
    visible: bool,
    inspecting: bool,
    hovered: Option<(Entity, Rect)>,
    described_hover: Option<Entity>,
    hover_target: Option<UiTarget>,
    selected: Option<Entity>,
    selected_at: Option<Vec2>,
    described_selected: Option<Entity>,
    target: Option<UiTarget>,
    click: Option<Vec2>,
    pending: Pending,
    editor_action: Option<EditorAction>,
    seq: u32,
    pending_pin: Option<(Entity, u32)>,
}

/// Adds the annotation overlay to the app.
///
/// Attach it only when the opt-in says so, for example
/// `if bevy_ui_debugger::is_requested() { app.add_plugins(UiDebuggerPlugin::default()); }`.
pub struct UiDebuggerPlugin {
    /// Key that shows or hides the whole overlay.
    pub toggle_key: KeyCode,
    /// Directory that receives `ui-debug-notes.jsonl`. Notes are also printed to stderr as one
    /// block when the game ends.
    pub log_dir: Option<PathBuf>,
    /// Font used by the overlay. Without it Bevy's default font is used when available.
    pub font: Option<Handle<Font>>,
    /// Absolute offsets of the toggle button; move it where the game has no UI of its own.
    pub button_position: UiRect,
}

impl Default for UiDebuggerPlugin {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F10,
            log_dir: None,
            font: None,
            button_position: UiRect::new(Val::Auto, Val::Px(12.0), Val::Auto, Val::Px(12.0)),
        }
    }
}

impl Plugin for UiDebuggerPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextInputPlugin>() {
            app.add_plugins(UiWidgetsPlugins);
        }
        app.init_resource::<OverlayState>()
            .init_resource::<NoteHistory>()
            .insert_resource(Config {
                toggle_key: self.toggle_key,
                font: self.font.clone(),
                button_position: self.button_position,
            })
            .insert_resource(NoteSink::open(self.log_dir.clone()))
            .add_systems(Startup, spawn_overlay)
            .add_systems(Last, print_notes_on_exit)
            .add_systems(PreUpdate, (read_hotkey, capture_click).chain())
            .add_systems(
                PreUpdate,
                // Inside `InputSystems`, so any game system ordered after that set sees
                // the cleaned input; an unordered game system may still run first.
                suppress_game_input
                    .in_set(InputSystems)
                    .after(keyboard_input_system)
                    .after(mouse_button_input_system)
                    .after(capture_click)
                    .before(InputFocusSystems::Dispatch),
            )
            .add_systems(
                Update,
                (
                    refresh_mode,
                    update_hover,
                    handle_click,
                    describe,
                    sync_editor,
                    update_pins,
                    update_tooltip,
                )
                    .chain(),
            );
    }
}

fn spawn_overlay(mut commands: Commands, config: Res<Config>, mut state: ResMut<OverlayState>) {
    let font = config.font.clone();
    let text_style = TextFont {
        font_size: FontSize::Px(FONT_PX),
        font: font_source(font.as_ref()),
        ..default()
    };
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(0.0),
                top: Val::Percent(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(i32::MAX),
            Pickable::IGNORE,
            OverlayNode,
            OverlayRoot,
        ))
        .with_children(|parent| {
            state.highlight = Some(
                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 0.72, 0.2, 0.12)),
                        BorderColor::all(ACCENT),
                        Outline {
                            color: ACCENT,
                            width: Val::Px(1.0),
                            offset: Val::ZERO,
                        },
                        Pickable::IGNORE,
                        Visibility::Hidden,
                        OverlayNode,
                        HighlightRect,
                    ))
                    .id(),
            );
            state.pins_root = Some(
                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(0.0),
                            top: Val::Percent(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        Pickable::IGNORE,
                        OverlayNode,
                        PinsRoot,
                    ))
                    .id(),
            );
            let tooltip = parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        padding: UiRect::all(Val::Px(8.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                    BorderColor::all(MUTED),
                    Pickable::IGNORE,
                    Visibility::Hidden,
                    OverlayNode,
                    Tooltip,
                ))
                .with_child((
                    Node::default(),
                    Text::new(""),
                    text_style.clone(),
                    TextColor(Color::WHITE),
                    TextLayout::no_wrap(),
                    OverlayNode,
                    TooltipLabel,
                ))
                .id();
            state.tooltip = Some(tooltip);
            let pencil = parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: config.button_position.left,
                        right: config.button_position.right,
                        top: config.button_position.top,
                        bottom: config.button_position.bottom,
                        width: Val::Px(52.0),
                        height: Val::Px(40.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        padding: UiRect::all(Val::Px(6.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(PANEL_BG),
                    BorderColor::all(ACCENT),
                    Outline {
                        color: ACCENT,
                        width: Val::Px(2.0),
                        offset: Val::ZERO,
                    },
                    Pickable {
                        should_block_lower: true,
                        is_hoverable: true,
                    },
                    PencilButton,
                    OverlayNode,
                ))
                .with_child((
                    Node::default(),
                    Text::new("UI"),
                    text_style,
                    TextColor(ACCENT),
                    TextLayout::new(Justify::Center, LineBreak::WordBoundary),
                    OverlayNode,
                    PencilLabel,
                ))
                .id();
            state.pencil = Some(pencil);
        })
        .id();
    state.root = Some(root);
    state.visible = true;
    info!(
        "bevy_ui_debugger overlay active: toggle with {:?}, click the corner button to annotate",
        config.toggle_key
    );
}

/// Prints the buffered notes as one block once the game is asked to exit.
fn print_notes_on_exit(mut exits: MessageReader<AppExit>, mut sink: ResMut<NoteSink>) {
    if exits.read().next().is_some() {
        sink.print_all();
    }
}

fn font_source(font: Option<&Handle<Font>>) -> FontSource {
    font.cloned().map_or_else(FontSource::default, FontSource::Handle)
}

fn label(
    font: Option<&Handle<Font>>,
    body: &str,
    size: f32,
    color: Color,
    wrap: bool,
) -> (Text, TextFont, TextColor, TextLayout) {
    let layout = if wrap {
        TextLayout::default()
    } else {
        TextLayout::no_wrap()
    };
    (
        Text::new(body),
        TextFont {
            font_size: FontSize::Px(size),
            font: font_source(font),
            ..default()
        },
        TextColor(color),
        layout,
    )
}

fn read_hotkey(
    mut reader: MessageReader<KeyboardInput>,
    config: Res<Config>,
    mut state: ResMut<OverlayState>,
) {
    for input in reader.read() {
        if input.state != ButtonState::Pressed || input.repeat {
            continue;
        }
        if input.key_code == config.toggle_key {
            state.pending.toggle = true;
            state.pending.swallow = true;
        } else if input.logical_key == Key::Escape
            && state.visible
            && state.inspecting
            && state.editor_panel.is_none()
        {
            state.inspecting = false;
            state.selected = None;
            state.pending.swallow = true;
        }
    }
}

fn capture_click(
    mut reader: MessageReader<MouseButtonInput>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    buttons: Query<(&ComputedNode, &UiGlobalTransform), With<PencilButton>>,
    mut state: ResMut<OverlayState>,
) {
    let Ok((primary, window)) = windows.single() else {
        return;
    };
    for input in reader.read() {
        if input.window != primary
            || input.button != MouseButton::Left
            || input.state != ButtonState::Pressed
        {
            continue;
        }
        let Some(position) = window.cursor_position() else {
            continue;
        };
        state.click = Some(position);
        // The toggle button sits over the game, so its click must not reach the game either.
        let point = position * window.scale_factor();
        if state.visible
            && buttons
                .iter()
                .any(|(node, transform)| node.contains_point(*transform, point))
        {
            state.pending.swallow = true;
        }
    }
}

/// While inspecting, the game must not react to the keys and buttons used for annotation.
fn suppress_game_input(
    mut keys_code: ResMut<ButtonInput<KeyCode>>,
    mut keys: ResMut<ButtonInput<Key>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut state: ResMut<OverlayState>,
) {
    let swallow = std::mem::take(&mut state.pending.swallow);
    if !state.inspecting && !swallow {
        return;
    }
    let held_codes: Vec<KeyCode> = keys_code
        .get_pressed()
        .copied()
        .filter(|code| is_modifier_code(*code))
        .collect();
    let held_keys: Vec<Key> = keys
        .get_pressed()
        .filter(|key| is_modifier_key(key))
        .cloned()
        .collect();
    keys_code.clear();
    for code in held_codes {
        keys_code.press(code);
    }
    keys.clear();
    for key in held_keys {
        keys.press(key);
    }
    mouse.clear();
}

fn is_modifier_code(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    )
}

fn is_modifier_key(key: &Key) -> bool {
    matches!(
        key,
        Key::Control | Key::Alt | Key::Shift | Key::Super | Key::Meta | Key::Hyper
    )
}

fn refresh_mode(
    mut state: ResMut<OverlayState>,
    mut roots: Query<(&mut Visibility, &mut Pickable), With<OverlayRoot>>,
    mut pencil: Query<&mut BackgroundColor, With<PencilButton>>,
    mut pencil_label: Query<&mut TextColor, With<PencilLabel>>,
) {
    if state.pending.toggle {
        state.pending.toggle = false;
        state.visible = !state.visible;
        if !state.visible {
            state.inspecting = false;
            state.selected = None;
        }
    }
    let visible = state.visible;
    let inspecting = state.inspecting;
    for (mut visibility, mut pickable) in &mut roots {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        *pickable = if inspecting {
            Pickable {
                should_block_lower: true,
                is_hoverable: true,
            }
        } else {
            Pickable::IGNORE
        };
    }
    for mut background in &mut pencil {
        background.0 = if inspecting { ACCENT } else { PANEL_BG };
    }
    for mut color in &mut pencil_label {
        color.0 = if inspecting { Color::BLACK } else { ACCENT };
    }
}

fn update_hover(
    mut state: ResMut<OverlayState>,
    stack: Res<UiStack>,
    windows: Query<&Window, With<PrimaryWindow>>,
    game_nodes: GameNodes,
    mut highlight: Query<(&mut Node, &mut Visibility), With<HighlightRect>>,
) {
    let hovered = if state.visible && state.inspecting {
        windows
            .single()
            .ok()
            .and_then(|window| {
                window
                    .cursor_position()
                    .zip(Some(window.scale_factor()))
            })
            .and_then(|(cursor, scale)| inspect::topmost_node_at(&stack, &game_nodes, cursor, scale))
    } else {
        None
    };
    if let Ok((mut node, mut visibility)) = highlight.single_mut() {
        match hovered {
            Some((_, rect)) => {
                node.left = Val::Px(rect.min.x);
                node.top = Val::Px(rect.min.y);
                node.width = Val::Px(rect.width());
                node.height = Val::Px(rect.height());
                *visibility = Visibility::Visible;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    state.hovered = hovered;
}

fn handle_click(
    mut state: ResMut<OverlayState>,
    stack: Res<UiStack>,
    windows: Query<&Window, With<PrimaryWindow>>,
    game_nodes: GameNodes,
    overlay_nodes: Query<(Entity, &ComputedNode, &UiGlobalTransform), With<OverlayNode>>,
    parents: Query<&ChildOf>,
) {
    let Some(cursor) = state.click.take() else {
        return;
    };
    if !state.visible {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let scale = window.scale_factor();
    let point = cursor * scale;
    let overlay_hit = stack.uinodes.iter().rev().copied().find(|entity| {
        overlay_nodes
            .get(*entity)
            .is_ok_and(|(_, node, transform)| node.contains_point(*transform, point))
    });
    if let Some(pencil) = state.pencil
        && overlay_hit.is_some_and(|hit| in_subtree(hit, pencil, &parents))
    {
        state.inspecting = !state.inspecting;
        state.selected = None;
        return;
    }
    if let Some(panel) = state.editor_panel
        && overlay_hit.is_some_and(|hit| in_subtree(hit, panel, &parents))
    {
        return;
    }
    if !state.inspecting {
        return;
    }
    match inspect::topmost_node_at(&stack, &game_nodes, cursor, scale) {
        Some((entity, _)) => {
            state.selected = Some(entity);
            state.selected_at = Some(cursor);
        }
        None => state.selected = None,
    }
}

fn in_subtree(entity: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = Some(entity);
    for _ in 0..32 {
        match current {
            Some(candidate) if candidate == root => return true,
            Some(candidate) => {
                current = parents.get(candidate).ok().map(ChildOf::parent);
            }
            None => return false,
        }
    }
    false
}

/// Exclusive step: builds identities for the hovered and selected nodes and applies the
/// editor decision so the heavy part runs only when something changed.
fn describe(world: &mut World) {
    let scale = inspect::primary_window_scale(world);
    let (hover, selected, hover_changed, selected_changed, action) = {
        let state = world.resource::<OverlayState>();
        let hover = state.hovered.map(|(entity, _)| entity);
        (
            hover,
            state.selected,
            hover != state.described_hover,
            state.selected != state.described_selected,
            state.editor_action,
        )
    };
    let hover_target = hover_changed
        .then(|| hover.and_then(|entity| inspect::describe(world, entity, scale)))
        .flatten();
    let target = selected_changed
        .then(|| selected.and_then(|entity| inspect::describe(world, entity, scale)))
        .flatten();
    {
        let mut state = world.resource_mut::<OverlayState>();
        if hover_changed {
            state.described_hover = hover;
            state.hover_target = hover_target;
        }
        if selected_changed {
            state.described_selected = selected;
            state.target = target;
        }
    }
    if let Some(action) = action {
        world.resource_mut::<OverlayState>().editor_action = None;
        apply_editor_action(world, action);
    }
}

fn apply_editor_action(world: &mut World, action: EditorAction) {
    let Some(input) = world.resource::<OverlayState>().editor_input else {
        return;
    };
    if action == EditorAction::Cancel {
        world.resource_mut::<OverlayState>().selected = None;
        return;
    }
    let text = world
        .get_entity(input)
        .ok()
        .and_then(|entity| entity.get::<EditableText>())
        .map_or_else(String::new, |editable| {
            editable.value().to_string().trim().to_string()
        });
    let Some(target) = world.resource::<OverlayState>().target.clone() else {
        return;
    };
    world.resource_mut::<OverlayState>().selected = None;
    if text.is_empty() {
        return;
    }
    let time_secs = world.resource::<Time>().elapsed_secs_f64();
    let window = inspect::primary_window_size(world).unwrap_or(Vec2::ZERO);
    let note = {
        let mut state = world.resource_mut::<OverlayState>();
        state.seq += 1;
        Note {
            seq: state.seq,
            time_secs,
            cursor: state.selected_at.unwrap_or(target.rect.center()),
            window,
            target,
            text,
        }
    };
    world.resource_mut::<NoteSink>().write(&note.to_line());
    world.resource_mut::<OverlayState>().pending_pin = Some((note.target.entity, note.seq));
    world.resource_mut::<NoteHistory>().notes.push(note);
}

fn sync_editor(
    mut commands: Commands,
    mut state: ResMut<OverlayState>,
    config: Res<Config>,
    mut focus: ResMut<InputFocus>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let ready = state.visible
        && state.inspecting
        && state.selected.is_some()
        && state
            .target
            .as_ref()
            .is_some_and(|target| Some(target.entity) == state.selected);
    if ready {
        if state.editor_panel.is_some() {
            return;
        }
        let Some(target) = state.target.clone() else {
            return;
        };
        let window = windows.single().map_or(Vec2::new(1280.0, 720.0), |window| {
            Vec2::new(window.width(), window.height())
        });
        let (x, y) = editor_placement(target.rect, window);
        let font = config.font.clone();
        let mut input_entity = Entity::PLACEHOLDER;
        let panel = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(x),
                    top: Val::Px(y),
                    width: Val::Px(EDITOR_WIDTH),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(8.0),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(PANEL_BG),
                BorderColor::all(ACCENT),
                // A separate root, so it needs its own z to stay above the game's layers.
                GlobalZIndex(i32::MAX),
                Pickable {
                    should_block_lower: true,
                    is_hoverable: true,
                },
                OverlayNode,
                EditorPanel,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Node::default(),
                    label(font.as_ref(), &target.detail(), FONT_PX, MUTED, true),
                    OverlayNode,
                ));
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            align_items: AlignItems::Stretch,
                            ..default()
                        },
                        OverlayNode,
                    ))
                    .with_children(|row| {
                        input_entity = row
                            .spawn((
                                Node {
                                    padding: UiRect::all(Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(FIELD_BG),
                                BorderColor::all(MUTED),
                                TextInput,
                                EditableText {
                                    allow_newlines: false,
                                    ..default()
                                },
                                TextLayout::no_wrap(),
                                TextFont {
                                    font_size: FontSize::Px(FONT_PX),
                                    font: font_source(font.as_ref()),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                TextCursorStyle::default(),
                                OverlayNode,
                                EditorInput,
                            ))
                            .observe(on_editor_key)
                            .id();
                        row.spawn((
                            Node::default(),
                            label(font.as_ref(), "Enter save / Esc cancel", FONT_PX, MUTED, false),
                            OverlayNode,
                        ));
                    });
            })
            .id();
        state.editor_panel = Some(panel);
        state.editor_input = Some(input_entity);
        focus.set(input_entity, FocusCause::Auto);
    } else if let Some(panel) = state.editor_panel.take() {
        if focus.get() == state.editor_input {
            focus.clear();
        }
        state.editor_input = None;
        commands.entity(panel).despawn();
    }
}

fn editor_placement(rect: Rect, window: Vec2) -> (f32, f32) {
    let below = rect.max.y + 8.0;
    let y = if below + EDITOR_HEIGHT <= window.y {
        below
    } else {
        (rect.min.y - EDITOR_HEIGHT - 8.0).max(8.0)
    };
    let x = rect.min.x.clamp(8.0, (window.x - EDITOR_WIDTH - 8.0).max(8.0));
    (x, y)
}

fn on_editor_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    inputs: Query<&EditableText>,
    keys: Res<ButtonInput<Key>>,
    mut state: ResMut<OverlayState>,
) {
    let input = &event.input;
    if input.state != ButtonState::Pressed {
        return;
    }
    let Ok(editable) = inputs.get(event.focused_entity) else {
        return;
    };
    if editable.is_composing() {
        return;
    }
    let modified = keys.pressed(Key::Control) || keys.pressed(Key::Super) || keys.pressed(Key::Alt);
    match &input.logical_key {
        Key::Enter if !modified => {
            event.propagate(false);
            state.editor_action = Some(EditorAction::Submit);
        }
        Key::Escape => state.editor_action = Some(EditorAction::Cancel),
        _ => {}
    }
}

fn update_pins(
    mut commands: Commands,
    mut state: ResMut<OverlayState>,
    config: Res<Config>,
    pins_root: Query<Entity, With<PinsRoot>>,
    targets: GameNodes,
    mut pins: Query<(Entity, &Pin, &mut Node)>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let scale = windows.single().map_or(1.0, Window::scale_factor);
    if let Ok(root) = pins_root.single()
        && let Some((target, seq)) = state.pending_pin.take()
    {
        commands
            .spawn((
                ChildOf(root),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(PIN_SIZE),
                    height: Val::Px(PIN_SIZE),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(Val::Px(PIN_SIZE / 2.0)),
                    ..default()
                },
                BackgroundColor(ACCENT),
                Pickable::IGNORE,
                Pin { target },
                OverlayNode,
            ))
            .with_child((
                Node::default(),
                label(config.font.as_ref(), &seq.to_string(), FONT_PX, Color::BLACK, false),
                OverlayNode,
            ));
    }
    for (entity, pin, mut node) in &mut pins {
        match targets.get(pin.target) {
            Ok((_, computed, transform, _, _)) => {
                let rect = inspect::node_rect(computed, transform, scale);
                // Sit just left of the target so the pin never covers its content.
                node.left = Val::Px((rect.min.x - PIN_SIZE - 4.0).max(0.0));
                node.top = Val::Px((rect.center().y - PIN_SIZE / 2.0).max(0.0));
            }
            Err(_) => {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn update_tooltip(
    state: ResMut<OverlayState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut tooltips: Query<(&ComputedNode, &mut Node, &mut Visibility), With<Tooltip>>,
    mut labels: Query<&mut Text, With<TooltipLabel>>,
) {
    let Ok((computed, mut node, mut visibility)) = tooltips.single_mut() else {
        return;
    };
    let Some(target) = state
        .hover_target
        .as_ref()
        .filter(|_| state.visible && state.inspecting && state.editor_panel.is_none())
    else {
        *visibility = Visibility::Hidden;
        return;
    };
    if let Ok(mut span) = labels.single_mut() {
        let text = target.summary();
        if span.0 != text {
            span.0 = text;
        }
    }
    *visibility = Visibility::Visible;
    let Some(window) = windows.single().ok() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let size = computed.size / window.scale_factor();
    let x = (cursor.x + 18.0).min(window.width() - size.x - 8.0).max(8.0);
    let y = (cursor.y + 22.0).min(window.height() - size.y - 8.0).max(8.0);
    node.left = Val::Px(x);
    node.top = Val::Px(y);
}
