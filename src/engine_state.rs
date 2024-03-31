use std::collections::HashMap;

use wgpu::{
    ComputePipeline, Device, Extent3d, PipelineLayout, PolygonMode, Queue, RenderPipeline, Sampler,
    ShaderModule, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, VertexBufferLayout,
};

use crate::{ecs::ecs::Res, voxel::model::Material};

pub type Map<K, V> = HashMap<K, V>;

#[derive(Default)]
pub struct EngineState {
    materials: Map<String, Res<Material>>,
    render_pipelines: Map<String, Res<RenderPipeline>>,
    compute_pipelines: Map<String, Res<ComputePipeline>>,
    textures: Map<String, Res<TextureWithView>>,
    samplers: Map<String, Res<Sampler>>,
}

#[derive(Debug)]
pub struct TextureWithView {
    pub texture: Texture,
    pub view: TextureView,
}

#[derive(Debug)]
pub enum EngineError {
    NameAlreadyExists,
}

impl std::error::Error for EngineError {}
impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Associated Unique Name is already in use for the type")
    }
}

impl EngineState {
    pub fn dispose_texture_by_name(&mut self, texture_name: &str) -> Option<Res<TextureWithView>> {
        self.textures.remove(texture_name)
    }

    pub fn get_sampler(&self, sampler_name: &str) -> Option<Res<Sampler>> {
        self.samplers.get(sampler_name).cloned()
    }

    pub fn create_texture(
        &mut self,
        texture_name: String,
        size: (u32, u32),
        format: TextureFormat,
        device: &Device,
    ) -> Result<Res<TextureWithView>, EngineError> {
        if self.textures.contains_key(&texture_name) {
            return Err(EngineError::NameAlreadyExists);
        }

        let (width, height) = size;
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: Some(&texture_name),
            size,
            mip_level_count: 1,
            sample_count: 1,
            format,
            dimension: TextureDimension::D2,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let texture = Res::new(TextureWithView { texture, view });

        self.textures.insert(texture_name, texture.clone());

        Ok(texture)
    }

    pub fn write_texture(&self, texture: &Texture, queue: &Queue, rgba: &[u8]) {
        let size = texture.size();
        let image_size_in_bytes = (size.width * size.height) as usize * 4;

        assert!(rgba.len() <= image_size_in_bytes);

        queue.write_texture(
            texture.as_image_copy(),
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.width),
                rows_per_image: Some(size.height),
            },
            size,
        );
    }

    pub fn create_sampler(
        &mut self,
        sampler_name: String,
        device: &Device,
    ) -> Result<Res<Sampler>, EngineError> {
        if self.samplers.contains_key(&sampler_name) {
            return Err(EngineError::NameAlreadyExists);
        }

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(format!("{sampler_name}_sampler").as_str()),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            // lod_min_clamp: 0.0,
            // lod_max_clamp: 100.0,
            // compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let sampler = Res::new(sampler);

        self.samplers.insert(sampler_name, sampler.clone());

        Ok(sampler)
    }

    pub fn create_render_pipeline(
        &mut self,
        pipeline_name: String,
        device: &Device,
        shader: &ShaderModule,
        fragment_entry_point: &str,
        vertex_entry_point: &str,
        polygon_mode: PolygonMode,
        layout: &PipelineLayout,
        color_format: TextureFormat,
        depth_format: Option<TextureFormat>,
        vertex_layouts: &[VertexBufferLayout],
    ) -> Result<Res<RenderPipeline>, EngineError> {
        if self.render_pipelines.contains_key(&pipeline_name) {
            return Err(EngineError::NameAlreadyExists);
        }
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("{pipeline_name}_render_pipeline").as_str()),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: vertex_entry_point,
                buffers: vertex_layouts,
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: fragment_entry_point,
                targets: &[Some(color_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let pipeline = Res::new(pipeline);
        self.render_pipelines
            .insert(pipeline_name, pipeline.clone());

        Ok(pipeline)
    }
}
