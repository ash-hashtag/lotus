use anyhow::{anyhow, Context};
use log::error;
use std::{
    io::{BufReader, Cursor},
    ops::Range,
    path::PathBuf,
};
use wgpu::{util::DeviceExt, BindGroup, RenderPass};

use crate::ecs::ecs::Res;

use super::{texture, vertex::ModelVertex};

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Res<Mesh>>,
    pub materials: Vec<Res<Material>>,
}

#[derive(Debug)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: texture::Texture,
    pub normal_texture: texture::Texture,
    pub bind_group: wgpu::BindGroup,
}

impl Material {
    pub fn default_material(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let diffuse_texture = texture::Texture::default_diffuse_texture(device, queue);
        let normal_texture = texture::Texture::default_normal_texture(device, queue);

        let name = "Default Material";

        Self::new(device, name, diffuse_texture, normal_texture, layout)
    }

    pub fn new(
        device: &wgpu::Device,
        name: impl Into<String>,
        diffuse_texture: texture::Texture,
        normal_texture: texture::Texture,
        layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let name: String = name.into();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
            label: Some(name.as_str()),
        });

        Self {
            name,
            diffuse_texture,
            normal_texture,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: Res<Material>,
}

impl Mesh {
    pub fn draw<'a>(
        &'a self,
        instances: Range<u32>,
        rp: &mut RenderPass<'a>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rp.set_bind_group(0, &self.material.bind_group, &[]);
        rp.set_bind_group(1, camera_bind_group, &[]);
        rp.set_bind_group(2, light_bind_group, &[]);
        rp.draw_indexed(0..self.num_elements, 0, instances);
    }
    pub fn draw_with_material<'a>(
        &'a self,
        material: &'a Material,
        instances: Range<u32>,
        rp: &mut RenderPass<'a>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        rp.set_bind_group(0, &material.bind_group, &[]);
        rp.set_bind_group(1, camera_bind_group, &[]);
        rp.set_bind_group(2, light_bind_group, &[]);
        rp.draw_indexed(0..self.num_elements, 0, instances);
    }
}

impl Model {
    pub async fn load_obj_model_from_file_path(
        file_path: PathBuf,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        default_material: Res<Material>,
    ) -> anyhow::Result<Self> {
        let parent_dir = file_path
            .parent()
            .with_context(|| anyhow!("Can't access parent dir path"))?;
        let mut materials = Vec::new();
        let mut meshes = Vec::new();

        let obj_text = tokio::fs::read_to_string(&file_path).await?;
        let obj_cursor = Cursor::new(obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);
        let (obj_models, obj_materials) = tobj::load_obj_buf_async(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| async move {
                let path = parent_dir.join(p);
                match tokio::fs::read_to_string(&path).await {
                    Ok(mat_text) => {
                        return tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
                    }
                    Err(err) => {
                        error!("Failed to load {:?} :{}", path, err);
                        return tobj::MTLLoadResult::Err(tobj::LoadError::OpenFileFailed);
                    }
                };
            },
        )
        .await?;

        for m in obj_materials? {
            let diffuse_texture_file_path = parent_dir.join(
                m.diffuse_texture
                    .with_context(|| anyhow!("No diffuse texture found"))?,
            );
            let diffuse_texture = texture::Texture::from_file_path(
                device,
                queue,
                diffuse_texture_file_path.as_path(),
                false,
            )
            .await?;

            let normal_texture = if let Some(normal_texture_path) = m.normal_texture {
                let normal_texture_file_path = parent_dir.join(normal_texture_path);

                texture::Texture::from_file_path(
                    device,
                    queue,
                    normal_texture_file_path.as_path(),
                    true,
                )
                .await?
            } else {
                texture::Texture::default_normal_texture(device, queue)
            };

            let material = Material::new(device, m.name, diffuse_texture, normal_texture, layout);

            materials.push(Res::new(material));
        }

        for m in obj_models {
            let mut vertices = Vec::with_capacity(m.mesh.positions.len() / 3);
            for i in 0..m.mesh.positions.len() / 3 {
                let vertex = ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                };

                vertices.push(vertex);
            }

            let indices = &m.mesh.indices;

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("vertex_buffer_{}", m.name).as_str()),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(format!("index_buffer_{}", m.name).as_str()),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            let mesh = Mesh {
                name: m.name,
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as _,
                material: materials
                    .get(m.mesh.material_id.unwrap_or(0))
                    .unwrap_or(&default_material)
                    .clone(),
            };

            meshes.push(Res::new(mesh));
        }

        Ok(Self { materials, meshes })
    }

    pub fn draw<'a>(
        &'a self,
        instances: Range<u32>,
        rp: &mut RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        for mesh in self.meshes.iter() {
            mesh.draw(instances.clone(), rp, camera_bind_group, light_bind_group);
        }
    }
}
