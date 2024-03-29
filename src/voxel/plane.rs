use super::vertex::ModelVertex;

pub struct Plane;

pub trait PrimitiveShape {
    fn indices() -> &'static [u16];

    fn vertices() -> &'static [ModelVertex];
}

const VERTICES: &[ModelVertex] = &[
    ModelVertex {
        position: [0.5, -0.5, 0.0],
        tex_coords: [1.0, 0.0],
    }, // A
    ModelVertex {
        position: [0.5, 0.5, 0.0],
        tex_coords: [1.0, 1.0],
    }, // B
    ModelVertex {
        position: [-0.5, 0.5, 0.0],
        tex_coords: [0.0, 1.0],
    }, // C
    ModelVertex {
        position: [-0.5, -0.5, 0.0],
        tex_coords: [0.0, 0.0],
    }, // D
];

const INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

impl PrimitiveShape for Plane {
    fn indices() -> &'static [u16] {
        &INDICES
    }

    fn vertices() -> &'static [ModelVertex] {
        &VERTICES
    }
}
