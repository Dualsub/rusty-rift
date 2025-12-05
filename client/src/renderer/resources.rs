use crate::renderer::{MaterialInstance, MaterialPipeline, StaticMesh, Texture};

pub enum Resource {
    Mesh(StaticMesh),
    Texture(Texture),
    MaterialPipeline(MaterialPipeline),
    MaterialInstance(MaterialInstance),
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum ResourcePoolType {
    Global = 0,
    Scene = 1,
}

#[derive(Clone, Copy)]
pub struct ResourceId {
    pub pool_type: ResourcePoolType,
    pub index: u32,
}

pub struct ResourcePool {
    pools: [Vec<Resource>; 2],
}

impl ResourcePool {
    pub fn new() -> Self {
        Self {
            pools: [vec![], vec![]],
        }
    }

    pub fn add_resource(&mut self, resource: Resource, pool_type: ResourcePoolType) -> ResourceId {
        let pool = &mut self.pools[pool_type as usize];
        let index = pool.len() as u32;
        pool.push(resource);

        ResourceId { pool_type, index }
    }

    pub fn get_resource(&self, id: ResourceId) -> Option<&Resource> {
        let pool = &self.pools[id.pool_type as usize];
        pool.get(id.index as usize)
    }

    pub fn get_material_pipeline(&self, id: ResourceId) -> Option<&MaterialPipeline> {
        match self.get_resource(id) {
            Some(resource) => match resource {
                Resource::MaterialPipeline(material_pipeline) => Some(material_pipeline),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_material_instance(&self, id: ResourceId) -> Option<&MaterialInstance> {
        match self.get_resource(id) {
            Some(resource) => match resource {
                Resource::MaterialInstance(material_instance) => Some(material_instance),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn get_mesh(&self, id: ResourceId) -> Option<&StaticMesh> {
        match self.get_resource(id) {
            Some(resource) => match resource {
                Resource::Mesh(mesh) => Some(mesh),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn clear_pool(&mut self, pool_type: ResourcePoolType) {
        let pool = &mut self.pools[pool_type as usize];
        pool.clear();
    }
}
