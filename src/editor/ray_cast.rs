pub use bevy::prelude::*;
use bevy::{
    math::{Vec3A, Vec3Swizzles},
    render::primitives::Aabb,
};

use crate::grass_spawner::GrassSpawner;

use super::draw_event::DrawEvent;
pub(super) struct RayCastPlugin;

impl Plugin for RayCastPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(check_collision_on_click)
            .add_system(update_camera_ray);
    }
}

fn check_collision_on_click(
    mut grass_chunk: Query<(&Transform, &Aabb, &mut GrassSpawner), Without<RayCamera>>,
    camera_source: Query<(&Transform, &RayCamera)>,
    mouse_presses: Res<Input<MouseButton>>,
    mut draw_events: EventWriter<DrawEvent>,
) {
    if !mouse_presses.pressed(MouseButton::Left) {
        return;
    }
    let (_camera_transform, raycast_camera) = camera_source.single();
    let click_ray = raycast_camera.ray.as_ref().unwrap();
    for (chunk_transform, aabb, mut grass) in grass_chunk.iter_mut() {
        let aabb_center = aabb.center.as_dvec3().as_vec3() + chunk_transform.translation;

        let grass_plane = Primitive3d::Plane {
            point: aabb_center,
            normal: Vec3::Y,
        };
        let res = intersects_primitive(&click_ray, grass_plane).unwrap();
        let intersection_point = (res - aabb_center).xz();
        let aabb_extends = aabb.half_extents.as_dvec3().as_vec3().xz().abs();
        if aabb_extends.x > intersection_point.x
            && -aabb_extends.x < intersection_point.x
            && aabb_extends.y > intersection_point.y
            && -aabb_extends.y < intersection_point.y
        {
            let positions = (Vec2::new(
                intersection_point.x / aabb_extends.x,
                intersection_point.y / aabb_extends.y,
            ) + Vec2::ONE)
                / 2.;
            // let image = grass.height_map.as_ref().unwrap().height_map.clone();
            let image = grass.density_map.as_ref().unwrap().density_map.clone();
            // need to mut deref grass at some point
            let mut _d = &mut grass.density_map;
            draw_events.send(DrawEvent::Draw { positions, image });
        }
    }
}

#[derive(Component, Default)]
pub struct RayCamera {
    pub ray: Option<Ray>,
}
fn update_camera_ray(
    mut ray_cam: Query<(&mut RayCamera, &Camera, &GlobalTransform)>,
    mut cursor: EventReader<CursorMoved>,
) {
    let Some(cursor_position) = cursor.iter().last() else {
        return;
    };
    let cusor_position = cursor_position.position;
    let (mut ray, cam, transform) = ray_cam.single_mut();
    let maybe_ray = ray_from_screenspace(cusor_position, cam, transform);
    if let Some(r) = maybe_ray {
        ray.ray = Some(r);
    } else {
        warn!("couldn't extract ray");
    }
}

pub struct Ray {
    pub(crate) origin: Vec3A,
    pub(crate) direction: Vec3A,
}

fn ray_from_screenspace(
    cursor_pos_screen: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Ray> {
    let view = camera_transform.compute_matrix();

    let (viewport_min, viewport_max) = camera.logical_viewport_rect()?;
    let screen_size = camera.logical_target_size()?;
    let viewport_size = viewport_max - viewport_min;
    let adj_cursor_pos =
        cursor_pos_screen - Vec2::new(viewport_min.x, screen_size.y - viewport_max.y);

    let projection = camera.projection_matrix();
    let far_ndc = projection.project_point3(Vec3::NEG_Z).z;
    let near_ndc = projection.project_point3(Vec3::Z).z;
    let cursor_ndc = (adj_cursor_pos / viewport_size) * 2.0 - Vec2::ONE;
    let ndc_to_world: Mat4 = view * projection.inverse();
    let near = ndc_to_world.project_point3(cursor_ndc.extend(near_ndc));
    let far = ndc_to_world.project_point3(cursor_ndc.extend(far_ndc));
    let ray_direction = far - near;
    Some(Ray {
        origin: near.into(),
        direction: ray_direction.normalize().into(),
    })
}

pub fn intersects_primitive(ray: &Ray, shape: Primitive3d) -> Option<Vec3> {
    match shape {
        Primitive3d::Plane {
            point: plane_origin,
            normal: plane_normal,
        } => {
            // assuming vectors are all normalized
            let denominator = plane_normal.dot(ray.direction.into());
            if denominator.abs() > f32::EPSILON {
                let point_to_point = plane_origin - Vec3::from(ray.origin);
                let intersect_dist = plane_normal.dot(point_to_point) / denominator;
                let intersect_position =
                    Vec3::from(ray.direction) * intersect_dist + Vec3::from(ray.origin);
                Some(intersect_position)
            } else {
                None
            }
        }
    }
}

pub enum Primitive3d {
    Plane { point: Vec3, normal: Vec3 },
}
