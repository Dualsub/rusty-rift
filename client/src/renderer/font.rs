use std::collections::HashMap;

use shared::math::*;

use crate::renderer::{RenderDevice, Texture, TextureDesc};

pub struct FontDesc {
    pub glyphs: HashMap<u32, Glyph>,
    pub atlas_desc: TextureDesc,
}

impl FontDesc {
    pub fn load(bytes: &[u8]) -> anyhow::Result<FontDesc> {
        let mut read_index: usize = 0;
        let mut tmp = [0u8; 4];

        // Glyph count
        tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
        let glyph_count = u32::from_le_bytes(tmp);
        read_index += 4;

        log::info!("Loading font with {} glyphs", glyph_count);

        let mut glyphs: HashMap<u32, Glyph> = HashMap::new();

        for _ in 0..glyph_count {
            // Unicode
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            let unicode = u32::from_le_bytes(tmp);
            read_index += 4;

            // Advance
            tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
            let advance = f32::from_le_bytes(tmp);
            read_index += 4;

            // Has bounds
            tmp[0..1].copy_from_slice(&bytes[read_index..read_index + 1]);
            let has_bounds = tmp[0] != 0;
            read_index += 1;

            let mut bounds = None;
            let mut uv_bounds = None;
            if has_bounds {
                // Plane bounds
                let mut plane_offset = [0f32; 2];
                let mut plane_size = [0f32; 2];

                for i in 0..2 {
                    tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
                    plane_offset[i] = f32::from_le_bytes(tmp);
                    read_index += 4;
                }
                for i in 0..2 {
                    tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
                    plane_size[i] = f32::from_le_bytes(tmp);
                    read_index += 4;
                }

                bounds = Some(Bounds {
                    offset: Vec2::new(plane_offset[0], plane_offset[1]),
                    size: Vec2::new(plane_size[0], plane_size[1]),
                });

                // UV bounds
                let mut uv_offset = [0f32; 2];
                let mut uv_size = [0f32; 2];

                for i in 0..2 {
                    tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
                    uv_offset[i] = f32::from_le_bytes(tmp);
                    read_index += 4;
                }
                for i in 0..2 {
                    tmp.copy_from_slice(&bytes[read_index..read_index + 4]);
                    uv_size[i] = f32::from_le_bytes(tmp);
                    read_index += 4;
                }

                uv_bounds = Some(Bounds {
                    offset: Vec2::new(uv_offset[0], uv_offset[1]),
                    size: Vec2::new(uv_size[0], uv_size[1]),
                });
            }

            let glyph = Glyph {
                unicode,
                advance,
                plane: bounds,
                uv: uv_bounds,
            };

            log::info!("Loaded glyph: {:?}", glyph);

            glyphs.insert(unicode, glyph);
        }

        let atlas_desc = TextureDesc::load(&bytes[read_index..])?;

        Ok(FontDesc { glyphs, atlas_desc })
    }
}

#[derive(Debug)]
pub struct Bounds {
    offset: Vec2,
    size: Vec2,
}

#[derive(Debug)]
struct Glyph {
    unicode: u32,
    advance: f32,
    plane: Option<Bounds>,
    uv: Option<Bounds>,
}

pub struct Font {
    pub glyphs: HashMap<u32, Glyph>,
    pub atlas: Texture,
}

impl RenderDevice {
    pub fn load_font(&self, bytes: &[u8]) -> anyhow::Result<Font> {
        let desc = FontDesc::load(bytes)?;

        let atlas = self.create_texture(&desc.atlas_desc);

        Ok(Font {
            glyphs: desc.glyphs,
            atlas,
        })
    }
}
