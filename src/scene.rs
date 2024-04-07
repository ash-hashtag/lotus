use wgpu::{ComputePipeline, RenderPipeline, Sampler, Texture};

use crate::{ecs::ecs::Res, engine_state::TextureWithView};

#[derive(Default)]
pub struct Scene {
    pub textures: Vec<Res<TextureWithView>>,
    pub samples: Vec<Res<Sampler>>,
    pub render_pipelines: Vec<Res<RenderPipeline>>,
    pub compute_pipelines: Vec<Res<ComputePipeline>>,
}
