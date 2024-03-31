use std::collections::HashMap;

use wgpu::{ComputePipeline, RenderPipeline};

use crate::{ecs::ecs::Res, voxel::model::Material};

pub struct EngineState {
    materials: HashMap<String, Res<Material>>,
    render_pipelines: HashMap<String, Res<RenderPipeline>>,
    compute_pipelines: HashMap<String, Res<ComputePipeline>>,
}
