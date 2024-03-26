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

        let render_pipeline = create_render_pipeline(
            &device,
            &layout,
            color_format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            shader,
            "Wireframe Render Pipeline",
        );
        Ok(Self { render_pipeline })
    }

    // pub fn draw_mesh_instanced<'a, 'b: 'a>(
    //     mesh: &'b Mesh,
    //     instances: Range<u32>,
    //     render_pass: &'a mut wgpu::RenderPass<'a>,
    //     camera_bind_group: &'a wgpu::BindGroup,
    // ) {
    //     render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    //     render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    //     render_pass.set_bind_group(1, camera_bind_group, &[]);
    //     render_pass.draw_indexed(0..mesh.num_elements, 0, instances);
    // }
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
