use wgpu::TextureUsages;

use crate::renderer::RenderDevice;

pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub layer_count: u32,
    pub channel_count: u32,
    pub bytes_per_channel: u32,
    pub mip_level_count: u32,
    pub format: Option<wgpu::TextureFormat>,
    pub pixels: Vec<u8>, // If empty, othing will be uploaded
    pub usage: wgpu::TextureUsages,
    pub view_dimension: wgpu::TextureViewDimension,
    pub aspect: wgpu::TextureAspect,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            layer_count: 1,
            channel_count: 1,
            bytes_per_channel: 1,
            mip_level_count: 1,
            pixels: vec![],
            format: None,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_dimension: wgpu::TextureViewDimension::D2Array,
            aspect: wgpu::TextureAspect::All,
        }
    }
}

impl TextureDesc {
    pub fn load(bytes: &[u8]) -> anyhow::Result<TextureDesc> {
        let mut desc = TextureDesc::default();

        let mut read_index: usize = 0;
        let mut tmp = [0u8; 4];

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        desc.width = u32::from_le_bytes(tmp);
        read_index += 4;

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        desc.height = u32::from_le_bytes(tmp);
        read_index += 4;

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        desc.layer_count = u32::from_le_bytes(tmp);
        read_index += 4;

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        desc.channel_count = u32::from_le_bytes(tmp);
        read_index += 4;

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        desc.bytes_per_channel = u32::from_le_bytes(tmp);
        read_index += 4;

        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        desc.mip_level_count = u32::from_le_bytes(tmp);
        read_index += 4;

        desc.pixels.resize(bytes.len() - read_index, 0);
        desc.pixels.copy_from_slice(&bytes[read_index..bytes.len()]);

        Ok(desc)
    }

    pub fn wgpu_format(&self) -> Result<wgpu::TextureFormat, String> {
        match self.bytes_per_channel {
            // We only support u8, f16 and f32 for now

            // u8
            1 => match self.channel_count {
                1 => Ok(wgpu::TextureFormat::R8Unorm),
                2 => Ok(wgpu::TextureFormat::Rg8Unorm),
                4 => Ok(wgpu::TextureFormat::Rgba8Unorm),
                _ => Err(format!(
                    "Unknown channel count for {}: {}",
                    self.bytes_per_channel, self.channel_count
                )),
            },

            // f16
            2 => match self.channel_count {
                1 => Ok(wgpu::TextureFormat::R16Float),
                2 => Ok(wgpu::TextureFormat::Rg16Float),
                4 => Ok(wgpu::TextureFormat::Rgba16Float),
                _ => Err(format!(
                    "Unknown channel count for {}: {}",
                    self.bytes_per_channel, self.channel_count
                )),
            },

            // f32
            4 => match self.channel_count {
                1 => Ok(wgpu::TextureFormat::R32Float),
                2 => Ok(wgpu::TextureFormat::Rg32Float),
                4 => Ok(wgpu::TextureFormat::Rgba32Float),
                _ => Err(format!(
                    "Unknown channel count for {}: {}",
                    self.bytes_per_channel, self.channel_count
                )),
            },

            _ => Err(format!(
                "Unknown bytes per channel: {}",
                self.bytes_per_channel
            )),
        }
    }
}

pub struct Texture {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl RenderDevice {
    pub fn load_texture(&self, bytes: &[u8]) -> anyhow::Result<Texture> {
        let desc = TextureDesc::load(bytes)?;
        Ok(self.create_texture(&desc))
    }

    pub fn create_texture(&self, desc: &TextureDesc) -> Texture {
        let format = desc
            .format
            .unwrap_or(desc.wgpu_format().expect("Unknown format"));

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: desc.layer_count,
            },
            mip_level_count: desc.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: desc.usage,
            view_formats: &[],
        });

        if desc.pixels.len() > 0 {
            let mut read_offset: usize = 0;
            for mip_index in 0..desc.mip_level_count {
                let mip_width = desc.width >> mip_index;
                let mip_height = desc.height >> mip_index;

                assert_ne!(mip_width, 0);
                assert_ne!(mip_height, 0);

                let bytes_per_row = desc.bytes_per_channel * desc.channel_count * mip_width;

                let upload_size: usize = (bytes_per_row * mip_height * desc.layer_count) as usize;
                let read_end = read_offset + upload_size;
                let mip_pixels = &desc.pixels[read_offset..read_end];

                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip_index,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    // The actual pixel data
                    mip_pixels,
                    // The layout of the texture
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(mip_height),
                    },
                    wgpu::Extent3d {
                        width: mip_width,
                        height: mip_height,
                        depth_or_array_layers: desc.layer_count,
                    },
                );

                read_offset += upload_size;
            }
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            dimension: Some(desc.view_dimension),
            format: Some(format),
            array_layer_count: Some(desc.layer_count),
            aspect: desc.aspect,
            base_array_layer: 0,
            base_mip_level: 0,
            mip_level_count: Some(desc.mip_level_count),
            usage: Some(desc.usage),
        });

        Texture {
            _texture: texture,
            view,
        }
    }
}
