use std::hash::Hash;
use std::io::prelude::*;
use std::{collections::HashMap, fs::File};

use asset_importer::Scene;
use asset_importer::{Importer, mesh::Mesh, postprocess::PostProcessSteps};
use serde::{Deserialize, Serialize};

pub struct MeshLoadDesc<'a> {
    pub path: &'a str,
    pub output: &'a str,
    pub skeleton_output: Option<&'a str>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct BoneInfo {
    pub name: String,
    pub id: i32,
    pub parent_id: i32,
    pub offset_matrix: [f32; 16],
}

pub type BoneMap = HashMap<String, BoneInfo>;

pub fn load(desc: &MeshLoadDesc) -> std::io::Result<()> {
    let importer = Importer::new();
    let scene = importer
        .read_file(desc.path)
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::FLIP_UVS
                | PostProcessSteps::GEN_SMOOTH_NORMALS
                | PostProcessSteps::POPULATE_ARMATURE_DATA,
        )
        .import_file(desc.path)
        .expect("Could not import scene.");

    let is_skeletal = desc.skeleton_output.is_some();
    let mut bone_map = HashMap::new();
    if is_skeletal {
        bone_map = load_bones(&scene);
        let mut skeleton_file = File::create(desc.skeleton_output.unwrap())?;
        skeleton_file.write_all(serde_json::to_string_pretty(&bone_map)?.as_bytes());
    }

    let mut file = File::create(desc.output).expect("Could not open output file.");

    let meshes = scene.meshes();
    file.write_all(&(meshes.len() as u32).to_le_bytes())?;

    let mut total_vertex_count = 0u32;
    for (mesh_index, mesh) in meshes.enumerate() {
        let vertices = &mesh.vertices();
        let normals = &mesh.normals().expect("No normals.");
        let uvs = &mesh.texture_coords(0).expect("No UVs.");
        let colors = &mesh.vertex_colors(0);
        let (bone_weights, bone_ids) = if !bone_map.is_empty() {
            load_bone_weights(&mesh, &bone_map)
        } else {
            (Vec::new(), Vec::new())
        };

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

            if is_skeletal {
                for bone_id in bone_ids[vertex_index] {
                    file.write_all(&bone_id.to_le_bytes())?;
                }

                for bone_weight in bone_weights[vertex_index] {
                    file.write_all(&bone_weight.to_le_bytes())?;
                }
            }
        }
        let faces = mesh.faces();

        let num_indices = (faces.len() * 3) as u32;
        file.write_all(&(num_indices).to_le_bytes())?;

        for face in faces {
            assert_eq!(face.num_indices(), 3);
            for index in face.indices() {
                let index_value = *index + total_vertex_count;
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

    // We also write a bone info buffer
    if is_skeletal {
        // Topologically sorted bone buffer
        let mut bone_info: Vec<BoneInfo> = Vec::new();
        bone_info.resize_with(bone_map.len(), Default::default);
        for bone in bone_map.values() {
            bone_info[bone.id as usize] = bone.clone();
        }

        file.write_all(&(bone_info.len() as u32).to_le_bytes())?;
        for info in &bone_info {
            file.write_all(&info.id.to_le_bytes())?;
            file.write_all(&info.parent_id.to_le_bytes())?;
            for m in info.offset_matrix {
                file.write_all(&m.to_le_bytes())?;
            }
        }

        println!("Wrote {} bones.", bone_info.len());
    }

    Ok(())
}

fn load_bones(scene: &Scene) -> BoneMap {
    let mut bone_index = 0;
    let mut bone_map: BoneMap = HashMap::new();
    let mut bone_names: Vec<String> = Vec::new();

    for mesh in scene.meshes() {
        for bone in mesh.bones() {
            if bone_map.contains_key(&bone.name()) {
                continue;
            }

            bone_map.insert(
                bone.name(),
                BoneInfo {
                    name: bone.name(),
                    id: bone_index,
                    parent_id: -1,
                    offset_matrix: bone.offset_matrix().to_cols_array(),
                },
            );

            bone_index += 1;

            bone_names.push(bone.name());
        }
    }

    let root_node = &scene.root_node().unwrap();
    for bone_name in bone_names {
        let bone_node = root_node.find_node(&bone_name).unwrap();

        let mut current_parent = bone_node.parent();
        let mut parent_bone_id = -1;

        // We search for the paarent bone since there might be helper nodes in-between
        while let Some(parent_node) = current_parent {
            if let Some(parent_info) = bone_map.get(&parent_node.name()) {
                parent_bone_id = parent_info.id;
                break;
            } else {
                current_parent = parent_node.parent();
            }
        }

        if parent_bone_id == -1 {
            println!("{} has no bone ancestor parent", bone_name);
            continue;
        }

        let bone_info = bone_map.get_mut(&bone_name).unwrap();
        if bone_info.parent_id != -1 && bone_info.parent_id != parent_bone_id {
            println!(
                "Tried to set {} from {} to {}",
                bone_info.name, bone_info.parent_id, parent_bone_id
            );
        }

        bone_info.parent_id = parent_bone_id;
    }

    bone_map
}

fn load_bone_weights(mesh: &Mesh, bone_map: &BoneMap) -> (Vec<[f32; 4]>, Vec<[i32; 4]>) {
    let vertex_count = mesh.num_vertices() as usize;

    let mut weights = vec![[0.0f32; 4]; vertex_count];
    let mut ids = vec![[-1i32; 4]; vertex_count];

    for bone in mesh.bones() {
        let bone_id = bone_map.get(&bone.name()).unwrap().id;

        for vertex_weight in bone.weights() {
            let v = vertex_weight.vertex_id as usize;
            let w = vertex_weight.weight;

            // We try to put it in a free slot
            let mut free_slot: Option<usize> = None;
            for k in 0..4 {
                if ids[v][k] == -1 {
                    free_slot = Some(k);
                    break;
                }
            }

            if let Some(k) = free_slot {
                ids[v][k] = bone_id;
                weights[v][k] = w;
            } else {
                // If there is no free slot we keep the 4 largest weights
                let mut min_index = 0;
                let mut min_weight = weights[v][0];

                for k in 1..4 {
                    if weights[v][k] < min_weight {
                        min_weight = weights[v][k];
                        min_index = k;
                    }
                }

                if w > min_weight {
                    ids[v][min_index] = bone_id;
                    weights[v][min_index] = w;
                }
            }
        }
    }

    // Normalize weights so they sum to one
    for v in 0..vertex_count {
        let mut sum = 0.0;
        for k in 0..4 {
            sum += weights[v][k];
        }

        if sum > 0.0 {
            for k in 0..4 {
                weights[v][k] /= sum;
            }
        }
    }

    (weights, ids)
}
