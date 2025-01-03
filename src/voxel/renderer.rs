use egui_wgpu::RenderState;
use wgpu::{
    util::DeviceExt, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BufferUsages, ShaderStages,
};

use crate::{
    state::create_render_pipeline,
    voxel::{
        instance::InstanceRaw,
        model::Model,
        vertex::{ModelVertex, Vertex},
    },
};

use super::{instance::Instance, texture};

pub struct Renderer {
    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,

    index_buffer: wgpu::Buffer,
    num_indices: u32,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    depth_texture: texture::Texture,
}

impl Renderer {
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Self> {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("voxel_texture_bind_group_layout"),
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });
        let shader_code = tokio::fs::read_to_string("assets/shaders/voxel_shader.wgsl").await?;
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Model Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        };
        let shader = device.create_shader_module(shader);

        // let obj_model = Model::load_obj_model_from_file_path(
        //     "assets/models/plane_cube.obj".into(),
        //     device,
        //     queue,
        //     &texture_bind_group_layout,
        // )
        // .await?;

        todo!()
    }

    pub fn render(&mut self) {}
}
