use std::fs::File;
use std::io::prelude::*;

use crate::renderer::RenderDevice;

pub struct TextureDesc {
    width: u32,
    height: u32,
    layer_count: u32,
    channel_count: u32,
    bytes_per_channel: u32,
    mip_level_count: u32,
    pixels: Vec<u8>, // If empty, othing will be uploaded
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            layer_count: 1,
            channel_count: 3,
            bytes_per_channel: 1,
            mip_level_count: 1,
            pixels: vec![],
        }
    }
}

impl TextureDesc {
    pub fn load(path: &str) -> anyhow::Result<TextureDesc> {
        let mut file = File::create(path).expect("Could not open file");
        let mut desc = TextureDesc::default();

        let mut buf = [0u8; 4];

        file.read_exact(&mut buf)?;
        desc.width = u32::from_le_bytes(buf);
        file.read_exact(&mut buf)?;
        desc.height = u32::from_le_bytes(buf);
        file.read_exact(&mut buf)?;
        desc.layer_count = u32::from_le_bytes(buf);
        file.read_exact(&mut buf)?;
        desc.channel_count = u32::from_le_bytes(buf);
        file.read_exact(&mut buf)?;
        desc.bytes_per_channel = u32::from_le_bytes(buf);
        file.read_exact(&mut buf)?;
        desc.mip_level_count = u32::from_le_bytes(buf);
        file.read_to_end(&mut desc.pixels)?;

        Ok(desc)
    }

    pub fn wgpu_format(&self) -> Result<wgpu::TextureFormat, &'static str> {
        match self.bytes_per_channel {
            // We only support u8, f16 and f32 for now

            // u8
            1 => match self.channel_count {
                1 => Ok(wgpu::TextureFormat::R8Unorm),
                2 => Ok(wgpu::TextureFormat::Rg8Unorm),
                4 => Ok(wgpu::TextureFormat::Rgba8Unorm),
                _ => Err("Unknown format"),
            },

            // f16
            2 => match self.channel_count {
                1 => Ok(wgpu::TextureFormat::R16Float),
                2 => Ok(wgpu::TextureFormat::Rg16Float),
                4 => Ok(wgpu::TextureFormat::Rgba16Float),
                _ => Err("Unknown format"),
            },

            // f32
            4 => match self.channel_count {
                1 => Ok(wgpu::TextureFormat::R32Float),
                2 => Ok(wgpu::TextureFormat::Rg32Float),
                4 => Ok(wgpu::TextureFormat::Rgba32Float),
                _ => Err("Unknown format"),
            },

            _ => Err("Unknown format"),
        }
    }
}

pub struct Texture {
    texture: wgpu::Texture,
}

impl RenderDevice {
    pub fn load_texture(&self, path: &str) -> anyhow::Result<Texture> {
        let desc = TextureDesc::load(path)?;
        Ok(self.create_texture(&desc))
    }

    pub fn create_texture(&self, desc: &TextureDesc) -> Texture {
        let format = desc.wgpu_format().expect("Unknown format");
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: desc.layer_count,
            },
            mip_level_count: desc.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

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

        Texture { texture }
    }
}
