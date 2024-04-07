use std::collections::HashMap;

use wgpu::{
    ComputePipeline, Device, Extent3d, PipelineLayout, PolygonMode, Queue, RenderPipeline, Sampler,
    ShaderModule, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, VertexBufferLayout,
};

use crate::{ecs::ecs::Res, engine_state::TextureWithView, voxel::model::Material};

pub type Map<K, V> = HashMap<K, V>;

#[derive(Default)]
pub struct ClientState {
    materials: Map<String, Res<Material>>,
    render_pipelines: Map<String, Res<RenderPipeline>>,
    compute_pipelines: Map<String, Res<ComputePipeline>>,
    textures: Map<String, Res<TextureWithView>>,
    samplers: Map<String, Res<Sampler>>,
}
