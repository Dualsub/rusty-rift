use std::fs::File;
use std::io::prelude::*;

use image::{EncodableLayout, ImageReader, imageops};

fn mip_level_count(width: u32, height: u32) -> u32 {
    assert!(width > 0 && height > 0);

    let max_side = width.max(height);
    32 - max_side.leading_zeros()
}

pub fn load(path: &str, output: &str) -> anyhow::Result<()> {
    let img = ImageReader::open(path)?.with_guessed_format()?.decode()?;

    let width = img.width();
    let height = img.height();
    let layer_count: u32 = 1;
    let color = img.color();
    let channel_count = color.channel_count() as u32;
    let bytes_per_channel = (color.bytes_per_pixel() as u32) / channel_count;
    let mip_level_count = mip_level_count(width, height);

    println!(
        "Loaded {}x{}x{} of {:?}.",
        width, height, layer_count, color
    );

    let mut file = File::create(output).expect("Could not open output file.");

    // Header
    file.write_all(&width.to_le_bytes())?;
    file.write_all(&height.to_le_bytes())?;
    file.write_all(&layer_count.to_le_bytes())?;
    file.write_all(&channel_count.to_le_bytes())?;
    file.write_all(&bytes_per_channel.to_le_bytes())?;
    file.write_all(&mip_level_count.to_le_bytes())?;

    // Image
    for mip_index in 0..mip_level_count {
        let mip_width: u32 = width >> mip_index;
        let mip_height: u32 = height >> mip_index;
        for layer_index in 0..layer_count {
            println!("Layer: {}, Mip: {} {}", layer_index, mip_width, mip_height);
            if mip_width != width || mip_height != height {
                // We need to resize
                let mip =
                    imageops::resize(&img, mip_width, mip_height, imageops::FilterType::Lanczos3);
                file.write_all(mip.as_bytes())?;
            } else {
                // We are writing the whole image
                file.write_all(img.as_bytes())?;
            }
        }
    }

    println!(
        "Packed image into {}. Generated {} layers, each with {} mips",
        output, layer_count, mip_level_count
    );

    Ok(())
}
