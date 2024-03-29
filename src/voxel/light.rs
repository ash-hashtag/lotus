use cgmath::Vector3;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub color: [f32; 3],
    _padding: u32,
}

impl LightUniform {
    pub fn new(color: Vector3<f32>) -> Self {
        Self {
            color: color.into(),
            _padding: 0,
        }
    }
}
