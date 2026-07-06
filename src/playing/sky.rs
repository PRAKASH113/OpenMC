
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

/// Radius of the sky dome. Must stay comfortably inside the camera's far plane.
pub const SKY_DOME_RADIUS: f32 = 500.0;

#[derive(Clone, Copy, ShaderType)]
pub struct SkyUniform {
    pub zenith_color: Vec3,
    pub horizon_color: Vec3,
}

impl Default for SkyUniform {
    fn default() -> Self {
        Self {
            zenith_color: Vec3::new(0.24, 0.51, 0.86),
            horizon_color: Vec3::new(0.67, 0.80, 0.90),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub params: SkyUniform,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/sky.wgsl".into()
    }

    fn enable_shadows() -> bool {
        false
    }

    fn enable_prepass() -> bool {
        // Unlit and fragment-only: nothing else needs this material's depth/normal prepass output.
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // The camera sits inside this sphere, so both faces must render.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
