use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
    rc::Rc,
};

pub struct Res<T> {
    inner: Rc<T>,
}

impl<T> Deref for Res<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

pub struct ResId<T>(u64, PhantomData<T>);

impl<T: 'static> std::fmt::Debug for ResId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TypeId: {:?}, TypeName: {}, res_id: {}",
            TypeId::of::<T>(),
            std::any::type_name::<T>(),
            self.0
        )
    }
}
impl<T: 'static> Clone for ResId<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<T: 'static> Copy for ResId<T> {}

#[derive(Default)]
pub struct World {
    entities: HashMap<TypeId, HashMap<u64, Box<dyn Any>>>,
}

impl World {
    pub fn get<'a, T: 'static>(&'a self, rid: &ResId<T>) -> Option<&'a T> {
        let type_id = TypeId::of::<T>();
        let components = self.entities.get(&type_id)?;
        let component = components.get(&rid.0)?;
        let casted_component = component.downcast_ref::<T>()?;
        Some(casted_component)
    }

    pub fn insert<T: 'static>(&mut self, component: T) -> ResId<T> {
        let type_id = TypeId::of::<T>();
        let components = self.entities.entry(type_id).or_insert(HashMap::new());
        let res_id = ResId(rand::random(), PhantomData::default());
        components.insert(res_id.0, Box::new(component));

        res_id
    }

    pub fn remove<T: 'static>(&mut self, rid: ResId<T>) -> Option<Box<T>> {
        let type_id = TypeId::of::<T>();
        let components = self.entities.get_mut(&type_id)?;
        let component = components.remove(&rid.0)?;
        component.downcast::<T>().ok()
    }
}
