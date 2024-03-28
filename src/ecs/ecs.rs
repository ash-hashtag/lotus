use std::{collections::HashMap, marker::PhantomData};

use crate::voxel::model::{Material, Mesh, Model};

pub struct ResId<T> {
    uuid: usize,
    phantom_data: PhantomData<T>,
}

impl<T> std::hash::Hash for ResId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state)
    }
}

impl<T> PartialEq for ResId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl<T> Eq for ResId<T> {}

impl<T> PartialOrd for ResId<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.uuid.partial_cmp(&other.uuid)
    }
}

impl<T> Ord for ResId<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.uuid.cmp(&other.uuid)
    }
}

type ResourcePool<T> = HashMap<ResId<T>, T>;

pub struct EntityComponentSystem {
    materials: ResourcePool<Material>,
    meshes: ResourcePool<Mesh>,
    models: ResourcePool<Model>,
}

impl EntityComponentSystem {
    pub fn get_material(&self, id: ResId<Material>) -> Option<&Material> {
        self.materials.get(&id)
    }
    pub fn dispose_material(&mut self, id: ResId<Material>) -> Option<Material> {
        self.materials.remove(&id)
    }
    pub fn get_meshes(&self, id: ResId<Mesh>) -> Option<&Mesh> {
        self.meshes.get(&id)
    }
    pub fn dispose_mesh(&mut self, id: ResId<Mesh>) -> Option<Mesh> {
        self.meshes.remove(&id)
    }
    pub fn get_models(&self, id: ResId<Model>) -> Option<&Model> {
        self.models.get(&id)
    }
    pub fn dispose_model(&mut self, id: ResId<Model>) -> Option<Model> {
        self.models.remove(&id)
    }
}
