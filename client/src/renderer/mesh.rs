use shared::math::Mat4;

use crate::renderer::{Buffer, BufferDesc, RenderDevice};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StaticMeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uvs: [f32; 3],
    pub color: [f32; 4],
}

impl StaticMeshVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<StaticMeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkeletalMeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uvs: [f32; 3],
    pub color: [f32; 4],
    pub bone_ids: [i32; 4],
    pub bone_weights: [f32; 4],
}

impl SkeletalMeshVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32x4, 4 => Sint32x4, 5 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkeletalMeshVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoneInfo {
    pub id: i32,
    pub parent_id: i32,
    pub offset_matrix: [f32; 16],
}

#[derive(Default)]
pub struct MeshLoadDesc {
    pub vertex_data: Vec<u8>,
    pub indices: Vec<u32>,
    pub _bones: Vec<BoneInfo>,
}

impl MeshLoadDesc {
    pub fn load(bytes: &[u8], vertex_size: usize) -> anyhow::Result<MeshLoadDesc> {
        let mut desc = MeshLoadDesc::default();

        let mut read_index: usize = 0;
        let mut tmp = [0u8; 4];

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        let mesh_count = u32::from_le_bytes(tmp);
        read_index += 4;

        for _ in 0..mesh_count {
            // Vertex data read
            {
                tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
                let vertex_count = u32::from_le_bytes(tmp);
                read_index += 4;

                let vertex_data_size = vertex_count as usize * vertex_size;

                let write_start = desc.vertex_data.len();
                desc.vertex_data.resize(write_start + vertex_data_size, 0);

                let read_end = read_index + vertex_data_size;
                desc.vertex_data[write_start..write_start + vertex_data_size]
                    .copy_from_slice(&bytes[read_index..read_end]);
                read_index += vertex_data_size;
            }

            // Index data read
            {
                tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
                let index_count = u32::from_le_bytes(tmp);
                read_index += 4;

                let index_data_size = index_count as usize * std::mem::size_of::<u32>();
                let write_start = desc.indices.len();
                desc.indices.resize(write_start + index_count as usize, 0);

                let read_end = read_index + index_data_size;

                // Collect into an aligned Vec<u32>
                let tmp_indices: Vec<u32> =
                    bytemuck::pod_collect_to_vec(&bytes[read_index..read_end]);

                desc.indices[write_start..write_start + index_count as usize]
                    .copy_from_slice(&tmp_indices);

                read_index += index_data_size;
            }
        }

        // If there are more bytes to read, there is a bone buffer
        if read_index < bytes.len() {
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            let bone_count = u32::from_le_bytes(tmp) as usize;
            read_index += 4;

            const BONE_SIZE: usize = std::mem::size_of::<BoneInfo>();
            let read_end = read_index + bone_count * BONE_SIZE;
            desc._bones = bytemuck::pod_collect_to_vec(&bytes[read_index..read_end]);
        }

        Ok(desc)
    }
}

pub struct MeshDrawInfo<'a> {
    pub vertex_slice: wgpu::BufferSlice<'a>,
    pub index_slice: wgpu::BufferSlice<'a>,
    pub index_count: u32,
}

pub struct StaticMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl StaticMesh {
    pub fn get_draw_info(&self) -> MeshDrawInfo<'_> {
        MeshDrawInfo {
            vertex_slice: self.vertex_buffer.buffer.slice(..),
            index_slice: self.index_buffer.buffer.slice(..),
            index_count: self.index_count,
        }
    }
}

pub struct SkeletalMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    pub bones: Vec<BoneInfo>,
}

impl SkeletalMesh {
    pub fn get_draw_info(&self) -> MeshDrawInfo<'_> {
        MeshDrawInfo {
            vertex_slice: self.vertex_buffer.buffer.slice(..),
            index_slice: self.index_buffer.buffer.slice(..),
            index_count: self.index_count,
        }
    }
}

impl RenderDevice {
    pub fn load_mesh(&self, bytes: &[u8]) -> anyhow::Result<StaticMesh> {
        let desc = MeshLoadDesc::load(bytes, (3 + 3 + 3 + 4) * std::mem::size_of::<f32>())?;
        self.create_mesh(&desc)
    }

    pub fn load_skeletal_mesh(&self, bytes: &[u8]) -> anyhow::Result<SkeletalMesh> {
        let desc = MeshLoadDesc::load(
            bytes,
            (3 + 3 + 3 + 4 + 4) * std::mem::size_of::<f32>() + 4 * std::mem::size_of::<i32>(),
        )?; // Silly I know ;)
        self.create_skeletal_mesh(&desc)
    }

    fn create_mesh_buffers(&self, desc: &MeshLoadDesc) -> (Buffer, Buffer) {
        let vertex_buffer = self.create_buffer(&BufferDesc {
            size: desc.vertex_data.len(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        self.write_buffer(&vertex_buffer, desc.vertex_data.as_slice(), 0);

        let index_buffer = self.create_buffer(&BufferDesc {
            size: desc.indices.len() * std::mem::size_of::<u32>(),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        self.write_buffer(
            &index_buffer,
            bytemuck::cast_slice(desc.indices.as_slice()),
            0,
        );

        (vertex_buffer, index_buffer)
    }

    pub fn create_mesh(&self, desc: &MeshLoadDesc) -> anyhow::Result<StaticMesh> {
        let (vertex_buffer, index_buffer) = self.create_mesh_buffers(desc);
        Ok(StaticMesh {
            vertex_buffer,
            index_buffer,
            index_count: desc.indices.len() as u32,
        })
    }

    pub fn create_skeletal_mesh(&self, desc: &MeshLoadDesc) -> anyhow::Result<SkeletalMesh> {
        let (vertex_buffer, index_buffer) = self.create_mesh_buffers(desc);
        Ok(SkeletalMesh {
            vertex_buffer,
            index_buffer,
            index_count: desc.indices.len() as u32,
            bones: desc._bones.clone(),
        })
    }
}
