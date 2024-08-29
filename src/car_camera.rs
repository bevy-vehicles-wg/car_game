use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use crate::car_suspension::CarPhysics;
#[derive(Component)]
pub struct CameraFollow {
    pub camera_translation_speed: f32,
    pub fake_transform: Transform,
    pub distance_behind: f32,
}
pub fn camera_follow(
    time: Res<Time>,
    rapier_context: Res<RapierContext>,
    mut car_query: Query<(&mut CarPhysics, &mut Transform), Without<CameraFollow>>,
    mut camera_query: Query<(&mut CameraFollow, &mut Transform), Without<CarPhysics>>,
) {
    if let Ok((mut camera_follow, mut camera_transform)) = camera_query.get_single_mut() {
        println!("following camera...");
        if let Ok((car_physics, car_transform)) = car_query.get_single_mut() {
            camera_transform.translation = car_transform.translation + Vec3::new(0.0, 10.0, 15.0);
            camera_transform.look_at(car_transform.translation, car_transform.forward());
        }
    }
}
