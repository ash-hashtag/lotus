use anyhow::{anyhow, Context};
use log::error;
use std::{
    io::{BufReader, Cursor},
    ops::Range,
    path::PathBuf,
};
use wgpu::util::DeviceExt;

use crate::ecs::ecs::{ResId, World};

use super::{texture, vertex::ModelVertex};

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<ResId<Mesh>>,
    pub materials: Vec<ResId<Material>>,
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
    pub material: usize,
}

impl Model {
    pub async fn load_obj_model_from_file_path(
        file_path: PathBuf,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        world: &mut World,
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

            let material_id = world.insert(material);

            materials.push(material_id);
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
                    // normal: [
                    //     m.mesh.normals[i * 3],
                    //     m.mesh.normals[i * 3 + 1],
                    //     m.mesh.normals[i * 3 + 2],
                    // ],

                    // tangent: [0.0; 3],
                    // bitangent: [0.0; 3],
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
                material: m.mesh.material_id.unwrap_or(0),
            };

            let mesh_id = world.insert(mesh);

            meshes.push(mesh_id);
        }

        Ok(Self { materials, meshes })
    }
}

pub trait DrawModel<'a> {
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model_instanced(
        &mut self,
        model: &ResId<Model>,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
        world: &'a World,
    );

    fn draw_model_instanced_with_material(
        &mut self,
        model: &ResId<Model>,
        material: &ResId<Material>,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
        world: &'a World,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model_instanced(
        &mut self,
        model_id: &ResId<Model>,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
        world: &'a World,
    ) {
        let model = world.get(model_id).unwrap();
        for mesh_id in model.meshes.iter() {
            let mesh = world.get(mesh_id).unwrap();
            let material_id = &model.materials[mesh.material];
            let material = world.get(material_id).unwrap();
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &ResId<Model>,
        material: &ResId<Material>,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,

        world: &'a World,
    ) {
        let material = world.get(material).unwrap();
        let model = world.get(model).unwrap();
        for mesh_id in model.meshes.iter() {
            let mesh = world.get(mesh_id).unwrap();
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}
