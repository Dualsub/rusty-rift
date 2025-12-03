use std::fs::File;
use std::io::prelude::*;

use asset_importer::{Importer, postprocess::PostProcessSteps};

pub fn load(path: &str, output: &str) -> std::io::Result<()> {
    let importer = Importer::new();
    let scene = importer
        .read_file(path)
        .with_post_process(PostProcessSteps::TRIANGULATE | PostProcessSteps::FLIP_UVS)
        .import_file(path)
        .expect("Could not import scene.");

    let mut file = File::create(output).expect("Could not open output file.");

    let meshes = scene.meshes();
    file.write_all(&(meshes.len() as u32).to_le_bytes())?;

    let mut total_vertex_count = 0u32;
    for (mesh_index, mesh) in meshes.enumerate() {
        let vertices = &mesh.vertices();
        let normals = &mesh.normals().expect("No normals.");
        let uvs = &mesh.texture_coords(0).expect("No UVs.");
        let colors = &mesh.vertex_colors(0);

        assert_eq!(vertices.len(), normals.len());
        assert_eq!(vertices.len(), uvs.len());

        let uv_layer = mesh_index as f32;
        let uv_layer_bytes = &uv_layer.to_le_bytes();

        file.write_all(&(vertices.len() as u32).to_le_bytes())?;

        for vertex_index in 0..vertices.len() {
            let position = &vertices[vertex_index];
            let normal = &normals[vertex_index];
            let uv_coordinate = &uvs[vertex_index];

            file.write_all(&position.x.to_le_bytes())?;
            file.write_all(&position.y.to_le_bytes())?;
            file.write_all(&position.z.to_le_bytes())?;

            file.write_all(&normal.x.to_le_bytes())?;
            file.write_all(&normal.y.to_le_bytes())?;
            file.write_all(&normal.z.to_le_bytes())?;

            file.write_all(&uv_coordinate.x.to_le_bytes())?;
            file.write_all(&uv_coordinate.y.to_le_bytes())?;
            file.write_all(uv_layer_bytes)?;

            match &colors {
                Some(color_list) => {
                    let color_value = &color_list[vertex_index];
                    file.write_all(&color_value.x.to_le_bytes())?;
                    file.write_all(&color_value.y.to_le_bytes())?;
                    file.write_all(&color_value.z.to_le_bytes())?;
                    file.write_all(&color_value.w.to_le_bytes())?;
                }
                None => {
                    let default_color_component: f32 = 1.0;
                    let color_value_bytes = &default_color_component.to_le_bytes();
                    file.write_all(color_value_bytes)?;
                    file.write_all(color_value_bytes)?;
                    file.write_all(color_value_bytes)?;
                    file.write_all(color_value_bytes)?;
                }
            }
        }
        let faces = mesh.faces();

        let num_indices = (faces.len() * 3) as u32;
        file.write_all(&(num_indices).to_le_bytes())?;

        for face in faces {
            assert_eq!(face.num_indices(), 3);
            for index in face.indices() {
                let index_value = (*index + total_vertex_count);
                file.write_all(&index_value.to_le_bytes())?;
            }
        }

        println!(
            "Wrote {} vertices, {} indices.",
            vertices.len(),
            num_indices
        );

        total_vertex_count += vertices.len() as u32;
    }

    Ok(())
}
