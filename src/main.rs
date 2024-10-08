use std::sync::atomic::AtomicBool;

use bevy::prelude::*;

use bevy::asset::LoadState;
use bevy::color::palettes::basic::*;
use bevy::render::render_resource::{AddressMode, FilterMode, SamplerDescriptor};
use bevy::render::texture::ImageSampler;
use bevy::window::{WindowMode, WindowResolution};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier3d::prelude::*;
use car_camera::CameraFollow;
use car_controls::CarController;
use car_suspension::WheelInfo;
use rand::rngs::ThreadRng;
use rand::Rng;

pub mod car_camera;
pub mod car_controls;
pub mod car_suspension;
pub mod timer_text;
pub mod ui_management;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::Srgba(BLUE)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        position: WindowPosition::Centered(MonitorSelection::Primary),
                        resolution: WindowResolution::new(1920., 1080.),
                        //mode: WindowMode::BorderlessFullscreen,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin {
                    default_sampler: SamplerDescriptor {
                        mag_filter: FilterMode::Nearest,
                        min_filter: FilterMode::Nearest,
                        mipmap_filter: FilterMode::Nearest,
                        ..Default::default()
                    }
                    .into(),
                    ..default()
                }),
        )
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, setup_graphics)
        .add_systems(Startup, setup_physics)
        .add_systems(Update, car_suspension::update_car_suspension)
        .add_systems(Update, car_camera::camera_follow)
        .add_systems(
            Update,
            car_controls::car_controls.after(car_suspension::update_car_suspension),
        )
        .insert_resource(MapStatus { loaded: false })
        .add_systems(Update, setup_map)
        .add_systems(Startup, ui_management::initialize_dialogue)
        .add_systems(Update, timer_text::text_update_system)
        .insert_resource(timer_text::Completion {
            finished: false,
            started: false,
        })
        .init_resource::<AssetsLoading>()
        .run();
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-90.0, 500.0, 90.0)
                .looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
            ..Default::default()
        },
        CameraFollow {
            camera_translation_speed: 1000.,
            fake_transform: Transform::from_xyz(0., 0., 0.),
            distance_behind: 1.,
        },
    ));
}
const CAR_SIZE: Vec3 = Vec3::new(0.5, 0.3, 0.935);
pub fn setup_physics(
    asset_server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1000.0,
    });
    let x_shape: Handle<Mesh> = asset_server.load("racetrack.glb#Mesh0/Primitive0");
    loading.0.push(x_shape.untyped());

    // directional 'sun' light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::PI / 4.),
            ..default()
        },
        ..default()
    });

    let car_size = CAR_SIZE;

    let mut wheel_vec = Vec::new();

    for i in 0..4 {
        let wheel_entity = commands
            .spawn(SceneBundle {
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                scene: asset_server.load("wheel.glb#Scene0"),
                ..default()
            })
            .id();

        wheel_vec.push(WheelInfo {
            entity: wheel_entity,
            hit: false,
        });
    }

    commands
        .spawn((
            SceneBundle {
                transform: Transform::from_xyz(0., 1., 0.),
                scene: asset_server.load("car.glb#Scene0"),
                ..default()
            },
            //TransformBundle::from(Transform::from_xyz(0., 5., 0.)),
            RigidBody::Dynamic,
            Collider::cuboid(car_size.x, car_size.y, car_size.z),
        ))
        .insert(car_suspension::CarPhysics {
            wheels_stationary_animation_speed: 10.,
            wheels_animation_speed: 3.,
            wheel_infos: wheel_vec,
            plane: Vec3::ZERO,
            car_size: CAR_SIZE,
            car_transform_camera: Transform::from_xyz(0., 0., 0.),
        })
        .insert(CarController {
            car_linear_damping: 0.5,
            rotate_to_rotation: Quat::IDENTITY,
            slerp_speed: 5.,
            rotated_last_frame: false,
            center_of_mass_altered: false,
            speed: 5000.,
            rotate_speed: 52.,
        })
        .insert(Velocity { ..default() })
        .insert(ExternalImpulse {
            impulse: Vec3::new(0., 0., 0.),
            torque_impulse: Vec3::new(0., 0., 0.),
        })
        .insert(ExternalForce {
            force: Vec3::new(0., 0., 0.),
            torque: Vec3::new(0., 0., 0.),
        })
        .insert(GravityScale(1.))
        .insert(Damping {
            linear_damping: 0.,
            angular_damping: 3.,
        })
        .insert(AdditionalMassProperties::MassProperties(MassProperties {
               mass: 2000.,
               local_center_of_mass: Vec3::new(0., -0.5, 0.),
               ..default()
        }))
        .insert(Ccd::enabled());
}

#[derive(Resource, Default)]
pub struct AssetsLoading(Vec<UntypedHandle>);

#[derive(Resource)]
pub struct MapStatus {
    pub loaded: bool,
}

fn setup_map(
    mut commands: Commands,
    mut map_status: ResMut<MapStatus>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh_handle: Handle<Mesh> = asset_server.load("racetrack.glb#Mesh0/Primitive0");

    if !map_status.loaded {
        if let Some(m) = meshes.get(&mesh_handle) {
            let mut map_mesh = m.clone();
            Mesh::generate_tangents(&mut map_mesh);

            let x_shape = Collider::from_bevy_mesh(m, &ComputedColliderShape::TriMesh).unwrap();
            if Collider::from_bevy_mesh(m, &ComputedColliderShape::TriMesh).is_none() {
                println!("{}", "the mesh failed to load");
            }
            let texture_handle = asset_server.load("sand.png");
            let normal_handle = asset_server.load("sand_normal.png");
            let ground_mat = materials.add(StandardMaterial {
                normal_map_texture: Some(normal_handle.clone()),
                base_color: Color::rgb(1.2, 1.2, 1.),
                perceptual_roughness: 0.5,
                base_color_texture: Some(texture_handle.clone()),
                cull_mode: None,
                unlit: false,
                ..default()
            });

            commands
                .spawn((
                    RigidBody::Fixed,
                    PbrBundle {
                        transform: Transform::from_xyz(0., 0., 0.)
                            .with_scale(Vec3::new(1., 1., 1.)),
                        mesh: meshes.add(map_mesh),
                        material: ground_mat,
                        ..default()
                    },
                ))
                .insert(x_shape);

            map_status.loaded = true;
            println!("just LOADED>>");
        }
    }
}
