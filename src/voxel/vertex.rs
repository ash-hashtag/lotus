use std::ops::Range;

use cgmath::Rotation3;
use wgpu::{util::DeviceExt, BindGroup, Device, Queue, RenderPass};

use super::{
    instance::{Instance, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW},
    model::Material,
    plane::PrimitiveShape,
};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

// Hexagon vertices
// pub const VERTICES: &[ModelVertex] = &[
//     ModelVertex {
//         position: [-0.0868241, 0.49240386, 0.0],
//         tex_coords: [0.4131759, 0.00759614],
//     }, // A
//     ModelVertex {
//         position: [-0.49513406, 0.06958647, 0.0],
//         tex_coords: [0.0048659444, 0.43041354],
//     }, // B
//     ModelVertex {
//         position: [-0.21918549, -0.44939706, 0.0],
//         tex_coords: [0.28081453, 0.949397],
//     }, // C
//     ModelVertex {
//         position: [0.35966998, -0.3473291, 0.0],
//         tex_coords: [0.85967, 0.84732914],
//     }, // D
//     ModelVertex {
//         position: [0.44147372, 0.2347359, 0.0],
//         tex_coords: [0.9414737, 0.2652641],
//     }, // E
// ];

// pub const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

pub struct PrimitiveRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    num_elements: u32,
    num_instances: u32,
    instances: Vec<Instance>,
}

impl PrimitiveRenderer {
    pub fn new<T>(device: &Device, instances: Vec<Instance>) -> Self
    where
        T: PrimitiveShape,
    {
        let vertices = T::vertices();
        let indices = T::indices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Primitive Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Primitive Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_data: Vec<_> = instances.iter().map(Instance::to_raw).collect();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let num_elements = indices.len() as _;
        let num_instances = instances.len() as _;

        Self {
            instances,
            num_instances,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            num_elements,
        }
    }

    pub fn add_instance(&mut self, instance: Instance, queue: &Queue) {
        self.instances.push(instance);
        self.update_instances(queue);
    }

    pub fn update_instances(&mut self, queue: &Queue) {
        let instance_data: Vec<_> = self.instances.iter().map(Instance::to_raw).collect();
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
        self.num_instances = self.instances.len() as _;
    }

    pub fn draw_with_material<'a>(
        &'a self,
        material: &'a Material,
        rp: &mut RenderPass<'a>,
        camera_bind_group: &'a BindGroup,
        light_bind_group: &'a BindGroup,
    ) {
        self.draw_with_bind_groups(
            rp,
            &[&material.bind_group, camera_bind_group, light_bind_group],
        );
        // rp.set_vertex_buffer(1, self.instance_buffer.slice(..));
        // rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // rp.set_bind_group(0, &material.bind_group, &[]);
        // rp.set_bind_group(1, camera_bind_group, &[]);
        // rp.set_bind_group(2, light_bind_group, &[]);
        // rp.draw_indexed(0..self.num_elements, 0, 0..self.num_instances);
    }

    pub fn draw_with_bind_groups<'a>(
        &'a self,
        rp: &mut RenderPass<'a>,
        bind_groups: &[&'a BindGroup],
    ) {
        rp.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rp.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rp.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        for (idx, bind_group) in bind_groups.iter().enumerate() {
            rp.set_bind_group(idx as _, bind_group, &[]);
        }
        rp.draw_indexed(0..self.num_elements, 0, 0..self.num_instances);
    }
}

pub struct TriangleRenderer {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instances: Range<u32>,
    num_elements: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl ModelVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, ];
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
