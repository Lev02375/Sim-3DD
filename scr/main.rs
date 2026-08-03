use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_rapier3d::prelude::*;
use rand::prelude::*;
use std::f32::consts::PI;

// ═══════════════════════════════════════════════════════════════
// RESOURCES & COMPONENTS
// ═══════════════════════════════════════════════════════════════

#[derive(Resource)]
struct Score(i32);

#[derive(Component)]
struct Player {
    speed: f32,
}

#[derive(Component)]
struct CameraPivot;

#[derive(Component)]
struct Target;

// ═══════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Sim-3D".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(Score(0))
        .add_systems(Startup, (setup_cursor, spawn_player, spawn_light, spawn_ground, spawn_walls, spawn_targets, setup_ui))
        .add_systems(Update, (toggle_cursor, player_move, player_look, shoot, update_score))
        .run();
}

// ═══════════════════════════════════════════════════════════════
// CURSOR
// ═══════════════════════════════════════════════════════════════

fn setup_cursor(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

fn toggle_cursor(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Ok(mut window) = window_query.get_single_mut() {
            let is_locked = window.cursor.grab_mode == CursorGrabMode::Locked;
            window.cursor.grab_mode = if is_locked { CursorGrabMode::None } else { CursorGrabMode::Locked };
            window.cursor.visible = is_locked;
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// PLAYER
// ═══════════════════════════════════════════════════════════════

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player { speed: 8.0 },
        Transform::from_xyz(0.0, 2.0, 0.0),
        RigidBody::Dynamic,
        LockedAxes::ROTATION_LOCKED,
        Collider::capsule_y(0.9, 0.4),
        Velocity::zero(),
        Damping { linear_damping: 2.0, angular_damping: 2.0 },
    ))
    .with_children(|parent| {
        parent.spawn((CameraPivot, Transform::from_xyz(0.0, 0.8, 0.0)))
            .with_children(|pivot| {
                pivot.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::NEG_Z, Vec3::Y)));
            });
    });
}

fn player_move(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&Player, &mut Velocity, &Transform)>,
) {
    let Ok((player, mut velocity, transform)) = player_query.get_single_mut() else { return };

    let mut direction = Vec3::ZERO;
    let forward = transform.rotation * Vec3::NEG_Z;
    let right = transform.rotation * Vec3::X;

    if keyboard.pressed(KeyCode::KeyW) { direction += forward; }
    if keyboard.pressed(KeyCode::KeyS) { direction -= forward; }
    if keyboard.pressed(KeyCode::KeyA) { direction -= right; }
    if keyboard.pressed(KeyCode::KeyD) { direction += right; }

    direction.y = 0.0;
    if direction.length_squared() > 0.0 { direction = direction.normalize(); }

    let current_vel = velocity.linvel;
    let target_vel = direction * player.speed;
    velocity.linvel = Vec3::new(target_vel.x, current_vel.y, target_vel.z);
}

fn player_look(
    mut mouse_events: EventReader<bevy::input::mouse::MouseMotion>,
    mut pivot_query: Query<&mut Transform, With<CameraPivot>>,
    mut player_query: Query<&mut Transform, (With<Player>, Without<CameraPivot>)>,
) {
    let Ok(mut pivot) = pivot_query.get_single_mut() else { return };
    let Ok(mut player) = player_query.get_single_mut() else { return };

    let mut delta = Vec2::ZERO;
    for event in mouse_events.read() { delta += event.delta; }
    if delta.length_squared() == 0.0 { return; }

    let sensitivity = 0.003;
    player.rotate_y(-delta.x * sensitivity);

    let (_, rotation, translation) = pivot.to_scale_rotation_translation();
    let mut euler = rotation.to_euler(EulerRot::YXZ);
    euler.1 -= delta.y * sensitivity;
    euler.1 = euler.1.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
    pivot.rotation = Quat::from_euler(EulerRot::YXZ, euler.0, euler.1, euler.2);
    pivot.translation = translation;
}

// ═══════════════════════════════════════════════════════════════
// WORLD
// ═══════════════════════════════════════════════════════════════

fn spawn_light(mut commands: Commands) {
    commands.spawn((
        DirectionalLight { illuminance: 1500.0, shadows_enabled: true, ..default() },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PointLight { intensity: 200000.0, color: Color::srgb(0.8, 0.2, 0.9), ..default() },
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));
}

fn spawn_ground(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.15),
            metallic: 0.1,
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Collider::cuboid(25.0, 0.1, 25.0),
        RigidBody::Fixed,
    ));
}

fn spawn_walls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.5, 0.8),
        metallic: 0.6,
        perceptual_roughness: 0.3,
        ..default()
    });
    let walls = vec![
        (Vec3::new(-10.0, 1.5, -10.0), Vec3::new(1.0, 3.0, 8.0)),
        (Vec3::new(10.0, 1.5, -5.0), Vec3::new(1.0, 3.0, 10.0)),
        (Vec3::new(0.0, 1.5, -15.0), Vec3::new(12.0, 3.0, 1.0)),
        (Vec3::new(-5.0, 1.5, 5.0), Vec3::new(8.0, 3.0, 1.0)),
        (Vec3::new(8.0, 1.5, 8.0), Vec3::new(1.0, 3.0, 6.0)),
        (Vec3::new(-8.0, 1.5, 12.0), Vec3::new(6.0, 3.0, 1.0)),
    ];
    for (pos, size) in walls {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(pos),
            Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
            RigidBody::Fixed,
        ));
    }
}

fn spawn_targets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = thread_rng();
    for _ in 0..8 {
        let x = rng.gen_range(-20.0..20.0);
        let z = rng.gen_range(-20.0..20.0);
        let y = rng.gen_range(1.5..3.0);
        let color = Color::srgb(rng.gen_range(0.5..1.0), rng.gen_range(0.2..0.8), rng.gen_range(0.3..1.0));
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.5).mesh().ico(5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.8,
                perceptual_roughness: 0.2,
                ..default()
            })),
            Transform::from_xyz(x, y, z),
            Collider::ball(0.5),
            RigidBody::Fixed,
            Target,
        ));
    }
}

// ═══════════════════════════════════════════════════════════════
// SHOOTING
// ═══════════════════════════════════════════════════════════════

fn shoot(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    rapier_context: Res<RapierContext>,
    mut targets: Query<(Entity, &mut Transform), With<Target>>,
    mut score: ResMut<Score>,
    mut commands: Commands,
) {
    if !mouse.just_pressed(MouseButton::Left) { return; }

    let Ok(window) = windows.get_single() else { return; };
    let Ok((camera, camera_transform)) = camera_query.get_single() else { return; };

    let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let Some(ray) = camera.viewport_to_world(camera_transform, center) else { return; };

    if let Some((entity, _)) = rapier_context.cast_ray(
        ray.origin,
        *ray.direction,
        100.0,
        true,
        QueryFilter::default(),
    ) {
        if let Ok((target_entity, mut transform)) = targets.get_mut(entity) {
            score.0 += 10;
            transform.scale = Vec3::splat(0.1);
            commands.entity(target_entity).despawn();
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// UI
// ═══════════════════════════════════════════════════════════════

#[derive(Component)]
struct ScoreText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("Счёт: 0"),
            TextFont { font_size: 40.0, ..default() },
            TextColor(Color::srgb(0.0, 1.0, 0.5)),
            ScoreText,
        ));
    });

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
    ))
    .with_children(|parent| {
        parent.spawn((
            Text::new("WASD — движение | Мышь — обзор | ЛКМ — стрельба | ESC — курсор"),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
        ));
    });
}

fn update_score(score: Res<Score>, mut query: Query<&mut Text, With<ScoreText>>) {
    if score.is_changed() {
        for mut text in query.iter_mut() {
            text.0 = format!("Счёт: {}", score.0);
        }
    }
  }
          
