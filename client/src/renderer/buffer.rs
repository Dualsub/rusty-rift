use crate::renderer::RenderDevice;

pub struct BufferDesc {
    pub size: usize,
    pub usage: wgpu::BufferUsages,
}

pub struct Buffer {
    pub buffer: wgpu::Buffer,
}

impl RenderDevice {
    pub fn create_buffer(&self, desc: &BufferDesc) -> Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            usage: desc.usage,
            size: desc.size as u64,
            mapped_at_creation: false,
        });

        Buffer { buffer }
    }

    pub fn write_buffer(&self, buffer: &Buffer, data: &[u8], offset: usize) {
        self.queue.write_buffer(&buffer.buffer, offset as u64, data);
    }
}
