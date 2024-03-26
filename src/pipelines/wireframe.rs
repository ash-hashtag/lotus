use std::ops::Range;

use crate::{
    state::create_render_pipeline,
    voxel::{
        instance::InstanceRaw,
        model::{Material, Mesh, Model},
        texture,
        vertex::{ModelVertex, Vertex},
    },
};

pub struct WireframeRenderPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
}

impl WireframeRenderPipeline {
    pub async fn new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        color_format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        let shader_code = tokio::fs::read_to_string("assets/shaders/wireframe.wgsl").await?;
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Wireframe Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Wireframe Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        };

        let vertex_layouts = &[ModelVertex::desc(), InstanceRaw::desc()];
        let depth_format = texture::Texture::DEPTH_FORMAT;

        let label = "Wireframe Render Pipeline";

        let shader = device.create_shader_module(shader);
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(color_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Line,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Ok(Self { render_pipeline })
    }
}

pub trait DrawWireframe<'a> {
    fn draw_wireframe_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawWireframe<'a> for wgpu::RenderPass<'a> {
    fn draw_wireframe_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        for mesh in model.meshes.iter() {
            self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            self.set_bind_group(0, camera_bind_group, &[]);
            self.draw_indexed(0..mesh.num_elements, 0, instances.clone());
        }
    }
}
