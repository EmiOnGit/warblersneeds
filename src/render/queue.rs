use bevy::core_pipeline::core_3d::{Opaque3d, Opaque3dBinKey};
use bevy::core_pipeline::prepass::{
    DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass,
};
use bevy::pbr::{MeshPipelineKey, RenderMeshInstances};
use bevy::prelude::*;
use bevy::render::mesh::GpuMesh;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{BinnedRenderPhaseType, DrawFunctions, ViewBinnedRenderPhases};
use bevy::render::render_resource::{PipelineCache, SpecializedMeshPipelines};
use bevy::render::view::ExtractedView;

use crate::prelude::WarblerHeight;

use super::grass_pipeline::{GrassPipeline, GrassRenderKey};
use super::GrassDrawCall;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn queue_grass_buffers(
    opaque_3d_draw_functions: Res<DrawFunctions<Opaque3d>>,
    grass_pipeline: Res<GrassPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GrassPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_mesh_instances: Res<RenderMeshInstances>,
    meshes: Res<RenderAssets<GpuMesh>>,
    material_meshes: Query<(Entity, &WarblerHeight)>,
    mut phase: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        Has<DepthPrepass>,
        Has<NormalPrepass>,
        Has<MotionVectorPrepass>,
        Has<DeferredPrepass>,
    )>,
) {
    let draw_custom = opaque_3d_draw_functions.read().id::<GrassDrawCall>();
    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

    for (camera_entity, view, depth_prepass, normal_prepass, motion_prepass, deferred_prepass) in
        &mut views
    {
        let mut view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }
        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }
        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }
        if motion_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }
        let Some(phase) = phase.get_mut(&camera_entity) else {
            continue;
        };
        for (entity, height) in material_meshes.iter() {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(entity) else {
                continue;
            };

            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let mesh_key =
                view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());
            let mut grass_key = GrassRenderKey::from(mesh_key);
            grass_key.uniform_height = match height {
                WarblerHeight::Uniform(_) => true,
                WarblerHeight::Texture(_) => false,
            };
            let pipeline = pipelines
                .specialize(&pipeline_cache, &grass_pipeline, grass_key, &mesh.layout)
                .unwrap();
            phase.add(
                Opaque3dBinKey {
                    pipeline,
                    draw_function: draw_custom,
                    asset_id: mesh_instance.mesh_asset_id.into(),
                    material_bind_group_id: None,
                    lightmap_image: None,
                },
                entity,
                BinnedRenderPhaseType::BatchableMesh,
            );
        }
    }
}
