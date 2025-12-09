use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Write};

use asset_importer::{Importer, postprocess::PostProcessSteps};

use crate::mesh::BoneMap;

pub struct AnimationLoadDesc<'a> {
    pub path: &'a str,
    pub skeleton: &'a str,
    pub output: &'a str,
}

#[derive(Clone, Copy)]
pub struct AnimationFrame {
    pub position: [f32; 3],
    pub rotation: [f32; 4], // [w, x, y, z]
}

impl Default for AnimationFrame {
    fn default() -> Self {
        AnimationFrame {
            position: [0.0; 3],
            rotation: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

pub fn load(desc: &AnimationLoadDesc) {
    let importer = Importer::new();
    let scene = importer
        .read_file(desc.path)
        .with_post_process(
            PostProcessSteps::TRIANGULATE
                | PostProcessSteps::FLIP_UVS
                | PostProcessSteps::GEN_SMOOTH_NORMALS,
        )
        .import_file(desc.path)
        .expect("Could not import scene.");

    let skeleton_file = File::open(desc.skeleton).expect("Could not open skeleton file.");
    let reader = BufReader::new(skeleton_file);
    let bone_map: BoneMap =
        serde_json::from_reader(reader).expect("Could not deserialize skeleton");

    let animation = scene.animations().next().expect("No animations found.");

    let mut channel_map: HashMap<String, usize> = HashMap::new();
    for (i, channel) in animation.channels().enumerate() {
        channel_map.insert(channel.node_name().to_string(), i);
    }

    let num_bones = bone_map.len();

    let reference_channel = animation.channels().next().expect("No channels found.");
    let num_frames = reference_channel.num_position_keys();

    println!(
        "Animation has {} bones (skeleton) and {} frames (reference channel).",
        num_bones, num_frames
    );

    // frames[frame][bone]
    let mut frames: Vec<AnimationFrame> = vec![AnimationFrame::default(); num_frames * num_bones];

    for frame_index in 0..num_frames {
        let frame_slice = &mut frames[frame_index * num_bones..(frame_index + 1) * num_bones];

        for bone_info in bone_map.values() {
            let bone_index = bone_info.id as usize;

            let channel_index = match channel_map.get(&bone_info.name) {
                Some(idx) => *idx,
                None => {
                    continue;
                }
            };

            let channel = animation
                .channel(channel_index)
                .expect("Channel index out of range");

            let mut position = [0.0, 0.0, 0.0];
            let pos_count = channel.num_position_keys();
            if pos_count > 0 {
                let used = frame_index.min(pos_count - 1);
                let key = &channel.position_keys()[used];
                position[0] = key.value.x;
                position[1] = key.value.y;
                position[2] = key.value.z;
            }

            let mut rotation = [1.0, 0.0, 0.0, 0.0];
            let rot_count = channel.num_rotation_keys();
            if rot_count > 0 {
                let used = frame_index.min(rot_count - 1);
                let key = &channel.rotation_keys()[used];
                rotation[0] = key.value.w;
                rotation[1] = key.value.x;
                rotation[2] = key.value.y;
                rotation[3] = key.value.z;
            }

            frame_slice[bone_index] = AnimationFrame { position, rotation };
        }
    }

    let mut tps = animation.ticks_per_second();
    assert!(tps > 0.0);

    let times: Vec<f32> = reference_channel
        .position_keys()
        .iter()
        .map(|k| (k.time / tps) as f32)
        .collect();

    let mut file = File::create(desc.output).expect("Could not open output file.");

    file.write_all(&(num_bones as u32).to_le_bytes())
        .expect("Could not write num_bones");
    file.write_all(&(num_frames as u32).to_le_bytes())
        .expect("Could not write num_frames");

    for frame in &frames {
        for p in &frame.position {
            file.write_all(&p.to_le_bytes())
                .expect("Could not write position");
        }
        for r in &frame.rotation {
            file.write_all(&r.to_le_bytes())
                .expect("Could not write rotation");
        }
    }

    for time in &times {
        file.write_all(&time.to_le_bytes())
            .expect("Could not write time");
    }

    println!("Wrote {} frames ({} bones/frame).", num_frames, num_bones);
}
