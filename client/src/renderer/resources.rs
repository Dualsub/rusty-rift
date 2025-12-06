use std::collections::HashMap;

use crate::renderer::{MaterialInstance, MaterialPipeline, StaticMesh, Texture};

pub enum Resource {
    Mesh(StaticMesh),
    Texture(Texture),
    MaterialPipeline(MaterialPipeline),
    MaterialInstance(MaterialInstance),
}

pub type ResourceHandle = u64;

pub const fn get_handle(s: &str) -> ResourceHandle {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;

    let bytes = s.as_bytes();
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as ResourceHandle;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }

    hash
}

pub struct ResourcePool {
    resources: HashMap<ResourceHandle, Resource>,
}

impl ResourcePool {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn add_resource(&mut self, handle: ResourceHandle, resource: Resource) {
        self.resources.insert(handle, resource);
    }

    pub fn get_resource(&self, handle: ResourceHandle) -> Option<&Resource> {
        self.resources.get(&handle)
    }

    pub fn get_material_pipeline(&self, handle: ResourceHandle) -> Option<&MaterialPipeline> {
        match self.get_resource(handle) {
            Some(resource) => match resource {
                Resource::MaterialPipeline(material_pipeline) => Some(material_pipeline),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_material_instance(&self, handle: ResourceHandle) -> Option<&MaterialInstance> {
        match self.get_resource(handle) {
            Some(resource) => match resource {
                Resource::MaterialInstance(material_instance) => Some(material_instance),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_mesh(&self, handle: ResourceHandle) -> Option<&StaticMesh> {
        match self.get_resource(handle) {
            Some(resource) => match resource {
                Resource::Mesh(mesh) => Some(mesh),
                _ => None,
            },
            _ => None,
        }
    }
}
