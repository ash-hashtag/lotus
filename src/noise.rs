use std::ops::RangeInclusive;

use log::info;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferDescriptor, BufferUsages, CommandEncoder,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device, Extent3d,
    ImageCopyBuffer, MaintainResult, PipelineLayoutDescriptor, Queue, ShaderModuleDescriptor,
    ShaderStages, SubmissionIndex, Texture, TextureFormat,
};

use crate::{state::save_tmp_image, ui::renderer::UiNode};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct NoiseUniform {
    seed: f32,
    frequency: f32,
    offset: [f32; 2],
    size: [u32; 2],
}

impl NoiseUniform {
    pub fn new(seed: f32, frequency: f32, offset: (f32, f32), size: (u32, u32)) -> Self {
        assert!(size != (0, 0));
        Self {
            seed,
            frequency,
            offset: [offset.0, offset.1],
            size: [size.0, size.1],
        }
    }
}

impl NoiseUniform {
    pub fn size(&self) -> (u32, u32) {
        (self.size[0], self.size[1])
    }

    pub fn image_size_in_bytes(&self) -> u64 {
        let (width, height) = self.size();

        width as u64 * height as u64 * 4 * std::mem::size_of::<u8>() as u64
    }
}

impl UiNode for NoiseUniform {
    fn add_ui(&mut self, ui: &mut egui::Ui) {
        use std::fmt::Write;
        let mut string_buf = self.seed.to_string();
        ui.horizontal_centered(|ui| {
            ui.label("Seed");
            if ui
                .add(egui::TextEdit::singleline(&mut string_buf))
                .changed()
            {
                self.seed = string_buf.parse().unwrap_or(self.seed);
            }
        });
        string_buf.clear();
        write!(&mut string_buf, "{}", self.frequency).unwrap();
        ui.horizontal_centered(|ui| {
            ui.label("Frequency");
            if ui
                .add(egui::TextEdit::singleline(&mut string_buf))
                .changed()
            {
                self.frequency = string_buf.parse().unwrap_or(self.frequency);
            }
        });
        ui.horizontal_centered(|ui| {
            ui.label("Offset ");
            write!(&mut string_buf, "{}", self.offset[0]).unwrap();
            if ui
                .add(egui::TextEdit::singleline(&mut string_buf))
                .changed()
            {
                self.offset[0] = string_buf.parse().unwrap_or(self.offset[0]);
            }
            write!(&mut string_buf, "{}", self.offset[1]).unwrap();
            if ui
                .add(egui::TextEdit::singleline(&mut string_buf))
                .changed()
            {
                self.offset[1] = string_buf.parse().unwrap_or(self.offset[1]);
            }
        });
    }
}

pub struct NoiseGenerator {
    noise_compute_pipeline: ComputePipeline,
    noise_bind_group: BindGroup,
    noise_output_buffer: Buffer,
    noise_storage_buffer: Buffer,
    noise_uniform_buffer: Buffer,
    noise_uniform: NoiseUniform,
}

impl NoiseGenerator {
    pub async fn new(device: &Device, noise_uniform: NoiseUniform) -> anyhow::Result<Self> {
        let shader_code = tokio::fs::read_to_string("assets/shaders/noise.wgsl").await?;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Noise Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        let noise_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Noise Bind Group Layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Noise Pipeline Layout"),
            bind_group_layouts: &[&noise_bind_group_layout],
            push_constant_ranges: &[],
        });

        let noise_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("noise_uniform_buffer"),
            contents: bytemuck::cast_slice(&[noise_uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let noise_output_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("noise_output_buffer"),
            size: noise_uniform.image_size_in_bytes(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let noise_storage_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("noise_output_buffer"),
            size: noise_uniform.image_size_in_bytes(),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let noise_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("noise_bind_group"),
            layout: &noise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: noise_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: noise_storage_buffer.as_entire_binding(),
                },
            ],
        });

        let noise_compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("noise_compute_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "cm_main",
        });

        Ok(Self {
            noise_storage_buffer,
            noise_compute_pipeline,
            noise_bind_group,
            noise_output_buffer,
            noise_uniform_buffer,
            noise_uniform,
        })
    }

    pub fn update_uniform(&mut self, device: &Device, queue: &Queue, noise_uniform: NoiseUniform) {
        if bytemuck::cast_slice::<_, u8>(&[noise_uniform])
            == bytemuck::cast_slice::<_, u8>(&[self.noise_uniform])
        {
            return;
        }

        let new_image_size_in_bytes = noise_uniform.image_size_in_bytes();
        if new_image_size_in_bytes > self.noise_output_buffer.size() {
            self.noise_output_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("noise_output_buffer"),
                size: noise_uniform.image_size_in_bytes(),
                usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if new_image_size_in_bytes > self.noise_storage_buffer.size() {
            self.noise_storage_buffer = device.create_buffer(&BufferDescriptor {
                label: Some("noise_output_buffer"),
                size: noise_uniform.image_size_in_bytes(),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
        }

        self.noise_uniform = noise_uniform;
        queue.write_buffer(
            &self.noise_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.noise_uniform]),
        );
    }

    pub fn compute<'a>(&'a self, encoder: &mut CommandEncoder) {
        let (width, height) = self.noise_uniform.size();
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Noise Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.noise_compute_pipeline);
            compute_pass.set_bind_group(0, &self.noise_bind_group, &[]);
            compute_pass.dispatch_workgroups(width, height, 1);
        }
        {
            encoder.copy_buffer_to_buffer(
                &self.noise_storage_buffer,
                0,
                &self.noise_output_buffer,
                0,
                self.noise_uniform.image_size_in_bytes(),
            );
        }
    }

    pub fn async_read(&self) {
        self.noise_output_buffer
            .slice(0..self.noise_uniform.image_size_in_bytes())
            .map_async(wgpu::MapMode::Read, |result| {
                info!("NoiseGenerator MAP READ {:?}", result);
            });
    }

    pub fn read_and_save_to_img(&self) {
        {
            let view = self
                .noise_output_buffer
                .slice(0..self.noise_uniform.image_size_in_bytes())
                .get_mapped_range();

            save_tmp_image(self.noise_uniform.size(), view.as_ref());
        }

        self.noise_output_buffer.unmap();
    }

    pub fn copy_to_texture(&self, encoder: &mut CommandEncoder, texture: &Texture) {
        let (width, height) = self.noise_uniform.size();
        encoder.copy_buffer_to_texture(
            ImageCopyBuffer {
                buffer: &self.noise_storage_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(width * std::mem::size_of::<u8>() as u32 * 4),
                    rows_per_image: Some(height),
                },
            },
            texture.as_image_copy(),
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}
