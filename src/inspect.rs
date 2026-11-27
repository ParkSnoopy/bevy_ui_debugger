//! Hit testing over the UI stack and identity extraction for a single node.

use std::fmt::Write as _;

use bevy::{
    camera::visibility::InheritedVisibility,
    ecs::{
        component::{Component, Components},
        entity::Entity,
        hierarchy::{ChildOf, Children},
        query::{With, Without},
        system::Query,
        world::EntityRef,
    },
    math::{Rect, Vec2},
    prelude::*,
    reflect::TypeRegistry,
    ui::{CalculatedClip, ComputedNode, GlobalZIndex, Node, UiGlobalTransform, UiStack, ZIndex},
    window::{PrimaryWindow, Window},
};

/// Marker for nodes that belong to the debugger overlay itself. Hit testing over game UI
/// skips them.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverlayNode;

/// Game UI nodes that can be inspected, in the shape hit testing needs.
pub(crate) type GameNodes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ComputedNode,
        &'static UiGlobalTransform,
        // UI nodes never get `ViewVisibility`; Bevy's own UI picking uses this one.
        Option<&'static InheritedVisibility>,
        Option<&'static CalculatedClip>,
    ),
    (With<Node>, Without<OverlayNode>),
>;

/// Identity of a UI node at the moment it was annotated.
///
/// `rect` is in logical pixels with a top-left origin, matching `Window::cursor_position`.
#[derive(Clone, Debug)]
pub struct UiTarget {
    /// Entity that was annotated.
    pub entity: Entity,
    /// `index v generation` form of the entity, unique for one run of the game.
    pub id: String,
    /// `Name` component, when present.
    pub name: Option<String>,
    /// Type paths of the components that are not engine-internal, e.g. `my_game::ui::Card`.
    pub markers: Vec<String>,
    /// Own and descendant text, joined with `" / "`.
    pub text: Option<String>,
    /// Axis-aligned bounding box on screen, logical pixels.
    pub rect: Rect,
    /// `GlobalZIndex` or `ZIndex` of the node, when present.
    pub z_index: Option<i32>,
    /// Labels from the root to the parent of the node.
    pub ancestry: Vec<String>,
    /// Compact dump of the `Node` style.
    pub style: String,
}

impl UiTarget {
    /// One-line identity, used for the hover tooltip.
    pub fn summary(&self) -> String {
        let mut out = format!("#{}", self.id);
        if let Some(name) = &self.name {
            let _ = write!(out, " {name:?}");
        }
        if let Some(marker) = self.markers.first() {
            out.push(' ');
            out.push_str(short_path(marker));
        }
        if let Some(text) = &self.text {
            let _ = write!(out, " {text:?}");
        }
        let _ = write!(
            out,
            " {:.0}x{:.0} @ ({:.0},{:.0})",
            self.rect.width(),
            self.rect.height(),
            self.rect.min.x,
            self.rect.min.y
        );
        out
    }

    /// Multi-line identity, used as the editor header.
    pub fn detail(&self) -> String {
        let mut lines = vec![format!(
            "#{} {}",
            self.id,
            self.name.as_deref().unwrap_or("(unnamed)")
        )];
        if !self.markers.is_empty() {
            lines.push(self.markers.join(", "));
        }
        if let Some(text) = &self.text {
            lines.push(format!("text: {text}"));
        }
        lines.push(format!(
            "rect: {:.0}x{:.0} @ ({:.0},{:.0})  z: {}",
            self.rect.width(),
            self.rect.height(),
            self.rect.min.x,
            self.rect.min.y,
            self.z_index.map_or("-".to_string(), |z| z.to_string())
        ));
        if !self.ancestry.is_empty() {
            lines.push(format!("path: {}", self.ancestry.join(" > ")));
        }
        lines.join("\n")
    }
}

/// Axis-aligned logical rect of a laid-out node.
pub(crate) fn node_rect(node: &ComputedNode, transform: &UiGlobalTransform, scale: f32) -> Rect {
    Rect::from_center_half_size(transform.translation / scale, node.size / (2.0 * scale))
}

/// Topmost visible game node under the logical cursor position, innermost stack order first.
pub(crate) fn topmost_node_at(
    stack: &UiStack,
    nodes: &GameNodes,
    cursor: Vec2,
    scale: f32,
) -> Option<(Entity, Rect)> {
    let point = cursor * scale;
    stack.uinodes.iter().rev().find_map(|entity| {
        let Ok((entity, node, transform, visibility, clip)) = nodes.get(*entity) else {
            return None;
        };
        if visibility.is_some_and(|v| !v.get()) {
            return None;
        }
        if !node.contains_point(*transform, point) {
            return None;
        }
        if clip.is_some_and(|c| !c.contains_point(point)) {
            return None;
        }
        Some((entity, node_rect(node, transform, scale)))
    })
}

/// Builds the identity of a node. Returns `None` when the entity has no laid-out UI node.
pub(crate) fn describe(world: &World, entity: Entity, scale: f32) -> Option<UiTarget> {
    let entity_ref = world.get_entity(entity).ok()?;
    let node = entity_ref.get::<Node>()?;
    let computed = entity_ref.get::<ComputedNode>()?;
    let transform = entity_ref.get::<UiGlobalTransform>()?;
    let registry = world.resource::<AppTypeRegistry>().read();
    let components = world.components();

    Some(UiTarget {
        entity,
        id: format!("{}v{}", entity.index_u32(), entity.generation().to_bits()),
        name: entity_ref.get::<Name>().map(|n| n.as_str().to_string()),
        markers: markers_of(entity_ref, components, &registry),
        text: collect_text(world, entity),
        rect: node_rect(computed, transform, scale),
        z_index: entity_ref
            .get::<GlobalZIndex>()
            .map(|z| z.0)
            .or_else(|| entity_ref.get::<ZIndex>().map(|z| z.0)),
        ancestry: ancestry(world, entity),
        style: style_summary(node),
    })
}

fn markers_of(entity: EntityRef, components: &Components, registry: &TypeRegistry) -> Vec<String> {
    let mut markers = Vec::new();
    for id in entity.archetype().components() {
        let Some(info) = components.get_info(*id) else {
            continue;
        };
        let path = info
            .type_id()
            .and_then(|type_id| registry.get(type_id))
            .map_or_else(
                || info.name().to_string(),
                |registration| registration.type_info().type_path().to_string(),
            );
        // Without bevy's `debug` feature unreflected names are an angle-bracket placeholder.
        if path.starts_with('<') || is_engine_path(&path) {
            continue;
        }
        markers.push(path);
    }
    markers
}

fn is_engine_path(path: &str) -> bool {
    path.strip_prefix("bevy")
        .is_some_and(|rest| rest.starts_with("::") || rest.starts_with('_'))
}

fn short_path(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn collect_text(world: &World, entity: Entity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut queue = std::collections::VecDeque::from([(entity, 0usize)]);
    while let Some((current, depth)) = queue.pop_front() {
        let Ok(entity_ref) = world.get_entity(current) else {
            continue;
        };
        let own = entity_ref
            .get::<Text>()
            .map(|text| text.0.as_str())
            .or_else(|| entity_ref.get::<TextSpan>().map(|span| span.0.as_str()));
        if let Some(text) = own.map(str::trim).filter(|text| !text.is_empty()) {
            parts.push(text.to_string());
        }
        if depth < 3
            && let Some(children) = entity_ref.get::<Children>()
        {
            queue.extend(children.iter().map(|child| (child, depth + 1)));
        }
        if parts.len() >= 10 {
            break;
        }
    }
    (!parts.is_empty()).then(|| truncate(&parts.join(" / "), 120))
}

fn ancestry(world: &World, entity: Entity) -> Vec<String> {
    let mut path = Vec::new();
    let components = world.components();
    let registry = world.resource::<AppTypeRegistry>().read();
    let mut current = world
        .get_entity(entity)
        .ok()
        .and_then(|e| e.get::<ChildOf>())
        .map(ChildOf::parent);
    while let Some(parent) = current {
        let Ok(entity_ref) = world.get_entity(parent) else {
            break;
        };
        path.push(node_label(entity_ref, components, &registry));
        current = entity_ref.get::<ChildOf>().map(ChildOf::parent);
        if path.len() >= 12 {
            break;
        }
    }
    path.reverse();
    path
}

fn node_label(entity: EntityRef, components: &Components, registry: &TypeRegistry) -> String {
    if let Some(name) = entity.get::<Name>() {
        return name.as_str().to_string();
    }
    markers_of(entity, components, registry)
        .first()
        .map_or_else(
            || format!("Node#{}", entity.id().index_u32()),
            |marker| short_path(marker).to_string(),
        )
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn style_summary(node: &Node) -> String {
    let rect = |r: &UiRect| format!("{:?} {:?} {:?} {:?}", r.left, r.top, r.right, r.bottom);
    format!(
        "display={:?} position={:?} size=({:?}, {:?}) min=({:?}, {:?}) max=({:?}, {:?}) \
         padding=[{}] margin=[{}] border=[{}] flex={:?} wrap={:?} grow={} basis={:?} \
         justify={:?} align={:?} gap=({:?}, {:?}) overflow={:?} inset=[{:?} {:?} {:?} {:?}]",
        node.display,
        node.position_type,
        node.width,
        node.height,
        node.min_width,
        node.min_height,
        node.max_width,
        node.max_height,
        rect(&node.padding),
        rect(&node.margin),
        rect(&node.border),
        node.flex_direction,
        node.flex_wrap,
        node.flex_grow,
        node.flex_basis,
        node.justify_content,
        node.align_items,
        node.row_gap,
        node.column_gap,
        node.overflow,
        node.left,
        node.top,
        node.right,
        node.bottom,
    )
}

/// Logical size of the primary window, or `None` when there is no primary window.
pub(crate) fn primary_window_size(world: &mut World) -> Option<Vec2> {
    let window = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .ok()?;
    Some(Vec2::new(window.width(), window.height()))
}

/// Scale factor of the primary window, or `1.0` when there is no primary window.
pub(crate) fn primary_window_scale(world: &mut World) -> f32 {
    world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(world)
        .map_or(1.0, Window::scale_factor)
}
