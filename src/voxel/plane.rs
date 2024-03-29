use cgmath::{Angle, InnerSpace, Quaternion, Rad, Rotation3, Vector3};
use wgpu::{util::DeviceExt, BindGroupDescriptor, Device, Queue, RenderPass};

use crate::ecs::ecs::{ResId, World};

use super::{
    instance::Instance,
    model::{DrawModel, Material, Mesh, Model},
    vertex::ModelVertex,
};

pub struct Plane {
    pub size: (f32, f32),
    pub normal: Vector3<f32>,
    pub position: Vector3<f32>,
    pub vertices: [ModelVertex; 4],
}

impl Plane {
    pub fn indices(&self) -> &'static [u32] {
        &[0u32, 1, 2, 2, 1, 3]
    }

    pub fn new(position: Vector3<f32>, size: (f32, f32), normal: Vector3<f32>) -> Self {
        let (x, y) = size;
        let vertices = [
            ModelVertex {
                position: [-x, y, 0.0], // Top left
                tex_coords: [1.0, 0.0],
            },
            ModelVertex {
                position: [x, y, 0.0], // Top right
                tex_coords: [1.0, 1.0],
            },
            ModelVertex {
                position: [-x, -y, 0.0], // Bottom  Left
                tex_coords: [0.0, 0.0],
            },
            ModelVertex {
                position: [x, -y, 0.0], // Bottom right
                tex_coords: [0.0, 1.0],
            },
        ];

        let normal = normal.normalize();

        Self {
            size,
            normal,
            vertices,
            position,
        }
    }

    pub fn vertices(&self) -> &[ModelVertex] {
        &self.vertices
    }
}

pub struct PlaneRenderer {
    plane: Plane,
    model: ResId<Model>,
    pub instances_buffer: wgpu::Buffer,
}

impl PlaneRenderer {
    pub fn new(
        plane: Plane,
        device: &Device,
        world: &mut World,
        default_material: ResId<Material>,
    ) -> Self {
        let indices = plane.indices();
        let vertices = plane.vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let position = plane.position;
        let up_vector = Vector3::unit_y();
        let rotation_axis = up_vector.cross(plane.normal);
        let rotation_angle = up_vector.dot(plane.normal).acos();

        let rotation = Quaternion::from_axis_angle(rotation_axis, Rad::acos(rotation_angle));

        let instance = Instance { position, rotation };
        let instances = [instance.to_raw()];
        let instances_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Plane Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mesh = Mesh {
            name: String::from("Plane Mesh"),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as _,
            material: 0,
        };

        let mesh = world.insert(mesh);
        let model = world.insert(Model {
            meshes: vec![mesh],
            materials: vec![default_material],
        });

        Self {
            plane,
            model,
            instances_buffer,
        }
    }

    pub fn draw<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
        world: &'a World,
    ) {
        render_pass.set_vertex_buffer(1, self.instances_buffer.slice(..));
        render_pass.draw_model_instanced(
            &self.model,
            0..1,
            camera_bind_group,
            light_bind_group,
            world,
        );
    }
}
