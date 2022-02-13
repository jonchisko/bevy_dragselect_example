use bevy::prelude::*; 
use std::fmt::{Display, Formatter};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .label(SelectionSystemLabels::Drag)
                .with_system(store_click_drag)
        )
        .add_system_set(
            SystemSet::new()
                .after(SelectionSystemLabels::Drag)
                .label(SelectionSystemLabels::SelectionAabb)
                .with_system(construct_aabb)
        )
        .add_system_set(
            SystemSet::new()
                .after(SelectionSystemLabels::SelectionAabb)
                .label(SelectionSystemLabels::SelectionElements)
                .with_system(consume_selection)
        )
        .add_event::<DragCompletedEvent>()
        .run();
}

#[derive(Default)]
struct ClickDrag {
    a_click: Option<(f32, f32)>,
    b_click: Option<(f32, f32)>,
}

impl Display for ClickDrag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (self.a_click, self.b_click) {
            (Some((x1, y1)), Some((x2, y2))) => {
                write!(f, "x1: {}, y1: {}; x2: {}, y2: {}", x1, y1, x2, y2)
            },
            _ => {
                write!(f, "Unfilled!")
            },
        }
    }
}

#[derive(Default)]
struct SomeSelection {
    selection: Option<SelectionAabb>,
}

struct SelectionAabb {
    center: (f32, f32),
    width: f32,
    height: f32,
}

impl SelectionAabb {
    fn new(a_click: (f32, f32), b_click: (f32, f32)) -> SelectionAabb {
        let delta_x = a_click.0 - b_click.0;
        let delta_y = a_click.1 - b_click.1;
        
        let mut center = b_click;
        center.0 += delta_x / 2.0;
        center.1 += delta_y / 2.0;

        SelectionAabb {
            center: center,
            width: delta_x.abs(),
            height: delta_y.abs(),
        }
    }
}

struct DragCompletedEvent;

#[derive(SystemLabel, Clone, Hash, PartialEq, Eq, Debug)]
enum SelectionSystemLabels {
    Drag,
    SelectionAabb,
    SelectionElements,
}

#[derive(Component)]
struct Selectable;

#[derive(Default)]
struct SpriteHandles {
    sad_handle: Handle<Image>,
    happy_handle: Handle<Image>,
}

#[derive(Component)]
struct MainCamera;


fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
    
    commands.insert_resource(ClickDrag {
        a_click: None,
        b_click: None,
    });
    commands.insert_resource(SomeSelection {
        selection: None,
    });
    commands.insert_resource(SpriteHandles {
        happy_handle: server.load("./sprites/happy.png"),
        sad_handle: server.load("./sprites/sad.png"),
    });

    let tmp_grid = (6, 6);
    for i in 0..24 {

        let x = i as f32 % tmp_grid.0 as f32;
        let y = (i / tmp_grid.1) as f32;
        commands.spawn()
            .insert_bundle(SpriteBundle{
                texture: server.load("./sprites/sad.png"),
                transform: Transform {
                    translation: Vec3::new(x * 64.0, y * 64.0, 0.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Selectable);
    }
}

fn store_click_drag(
    mut click_drag: ResMut<ClickDrag>,
    mut selection_aabb: ResMut<SomeSelection>,
    mut on_drag_completed: EventWriter<DragCompletedEvent>,
    mut selectable_items: Query<&mut Handle<Image>, With<Selectable>>,
    sprites: Res<SpriteHandles>,
    mouse_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    q_camera: Query<&Transform, With<MainCamera>>,
) {
    let mouse_world_pos = transform_to_world(&windows, &q_camera);
    if mouse_input.just_pressed(MouseButton::Left) {
        reset_selection(&mut click_drag, &mut selection_aabb, &mut selectable_items, &sprites);
        click_drag.a_click = Some((mouse_world_pos.unwrap().x, mouse_world_pos.unwrap().y));
    }
    
    if mouse_input.just_released(MouseButton::Left) {
        if click_drag.a_click.is_some() {
            click_drag.b_click = Some((mouse_world_pos.unwrap().x, mouse_world_pos.unwrap().y));
            on_drag_completed.send(DragCompletedEvent);
        }
    }
}

fn reset_selection(
    click_drag: &mut ResMut<ClickDrag>,
    selection_aabb: &mut ResMut<SomeSelection>,
    selectable_items: &mut Query<&mut Handle<Image>, With<Selectable>>,
    sprites: &Res<SpriteHandles>,
) {
    click_drag.a_click = None;
    click_drag.b_click = None;
    selection_aabb.selection = None;

    for mut sprite in selectable_items.iter_mut() {
        *sprite = sprites.sad_handle.clone();
    }
}

fn transform_to_world(
    wnds: &Res<Windows>,
    q_camera: &Query<&Transform, With<MainCamera>>
) -> Option<Vec4>
{
    let wnd = wnds.get_primary().unwrap();

    if let Some(pos) = wnd.cursor_position() {
        let wnd_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
        let p = pos - wnd_size / 2.0f32;
        let camera_transform = q_camera.single();
        return Some(camera_transform.compute_matrix() * p.extend(0.0).extend(1.0))
    }
    None 
}

fn construct_aabb(
    mut click_drag: ResMut<ClickDrag>,
    mut selection_aabb: ResMut<SomeSelection>,
    mut on_drag_completed: EventReader<DragCompletedEvent>
) {
    if on_drag_completed.iter().next().is_some() {
        let (a, b) = (click_drag.a_click.take().unwrap(), click_drag.b_click.take().unwrap());
        selection_aabb.selection = Some(SelectionAabb::new(a, b));
    }
}

fn consume_selection(
    mut selection_aabb: ResMut<SomeSelection>,
    mut selectable_items: Query<(&Transform, &mut Handle<Image>), With<Selectable>>,
    sprites: Res<SpriteHandles>,
) {
    let aabb = selection_aabb.selection.take();
    if aabb.is_some() {
        let aabb = aabb.unwrap();
        for (transform, mut handle_img) in selectable_items.iter_mut() {
            if in_aabb(transform, &aabb) {
                *handle_img = sprites.happy_handle.clone();
            }
        }
    }
}

fn in_aabb(t1: &Transform, t2: &SelectionAabb) -> bool {
    let pos = t1.translation;
    let (x1, y1, x2, y2) = 
    (
        t2.center.0 - t2.width / 2.0, 
        t2.center.1 - t2.height / 2.0,
        t2.center.0 + t2.width / 2.0,
        t2.center.1 + t2.height / 2.0,
    );
    pos.x >= x1 && pos.x <= x2 && pos.y >= y1 && pos.y < y2
}

