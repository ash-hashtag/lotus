use cgmath::Vector3;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    _padding: u32,
    pub color: [f32; 3],
    _padding2: u32,
}

impl LightUniform {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
            _padding: 0,
            _padding2: 0,
        }
    }
}
