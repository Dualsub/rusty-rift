use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Write};

use image::{EncodableLayout, ImageReader, imageops};

use crate::{font, texture};

pub struct FontLoadDesc<'a> {
    pub atlas: &'a str,
    pub json: &'a str,
    pub output: &'a str,
}

#[derive(Debug, Serialize, Deserialize)]
struct Bounds {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct FontGlyph {
    unicode: u32,
    advance: f32,
    #[serde(rename = "planeBounds")]
    plane_bounds: Option<Bounds>,
    #[serde(rename = "atlasBounds")]
    atlas_bounds: Option<Bounds>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FontAtlas {
    #[serde(rename = "type")]
    atlas_type: String,
    #[serde(rename = "distanceRange")]
    distance_range: u32,
    size: u32,
    width: u32,
    height: u32,
    #[serde(rename = "yOrigin")]
    y_origin: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FontMetrics {
    #[serde(rename = "emSize")]
    em_size: f32,
    #[serde(rename = "lineHeight")]
    line_height: f32,
    ascender: f32,
    descender: f32,
    #[serde(rename = "underlineY")]
    underline_y: f32,
    #[serde(rename = "underlineThickness")]
    underline_thickness: f32,
}

pub fn load(desc: &FontLoadDesc) -> anyhow::Result<()> {
    // Load talas image atlas
    let atlas =
        image::DynamicImage::ImageRgba8(ImageReader::open(desc.atlas)?.decode()?.to_rgba8());

    // Load font json
    let mut json_file = File::open(desc.json)?;
    let mut json_reader = BufReader::new(json_file);
    let font_json: serde_json::Value = serde_json::from_reader(json_reader)?;

    let glyphs: Vec<FontGlyph> = serde_json::from_value(font_json["glyphs"].clone())?;
    let font_atlas: FontAtlas = serde_json::from_value(font_json["atlas"].clone())?;
    let font_metrics: FontMetrics = serde_json::from_value(font_json["metrics"].clone())?;

    let file = &mut File::create(desc.output)?;

    file.write_all(&(glyphs.len() as u32).to_le_bytes())?;
    for glyph in glyphs.iter() {
        file.write_all(&glyph.unicode.to_le_bytes())?;
        file.write_all(&glyph.advance.to_le_bytes())?;
        match (&glyph.plane_bounds, &glyph.atlas_bounds) {
            (Some(plane_bounds), Some(atlas_bounds)) => {
                file.write_all(&1u8.to_le_bytes())?;

                let offset = [plane_bounds.left, plane_bounds.bottom];
                let size = [
                    plane_bounds.right - plane_bounds.left,
                    plane_bounds.top - plane_bounds.bottom,
                ];

                file.write_all(&offset[0].to_le_bytes())?;
                file.write_all(&offset[1].to_le_bytes())?;
                file.write_all(&size[0].to_le_bytes())?;
                file.write_all(&size[1].to_le_bytes())?;

                let uv = [
                    atlas_bounds.left / font_atlas.width as f32,
                    atlas_bounds.bottom / font_atlas.height as f32,
                ];
                let uv_size = [
                    (atlas_bounds.right - atlas_bounds.left) / font_atlas.width as f32,
                    (atlas_bounds.top - atlas_bounds.bottom) / font_atlas.height as f32,
                ];

                file.write_all(&uv[0].to_le_bytes())?;
                file.write_all(&uv[1].to_le_bytes())?;
                file.write_all(&uv_size[0].to_le_bytes())?;
                file.write_all(&uv_size[1].to_le_bytes())?;
            }
            _ => {
                file.write_all(&0u8.to_le_bytes())?;
            }
        }
    }

    texture::write_texture(&atlas, file)?;

    Ok(())
}
