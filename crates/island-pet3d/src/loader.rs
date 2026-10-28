use std::collections::VecDeque;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use gltf::mesh::Mode;

pub const MOMO_GLB: &[u8] = include_bytes!("../assets/momo.glb");

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub node_id: u32,
    pub material_id: u32,
}

impl Vertex {
    pub const LAYOUT: slint::wgpu_30::wgpu::VertexBufferLayout<'static> =
        slint::wgpu_30::wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: slint::wgpu_30::wgpu::VertexStepMode::Vertex,
            attributes: &slint::wgpu_30::wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Uint32,
                3 => Uint32,
            ],
        };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Root,
    Body,
    Arm,
    Cheek,
    Eye,
    Mouth,
    Spirit,
    Other,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub local: Mat4,
    pub rest_world: Mat4,
    pub kind: NodeKind,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Material {
    pub base_color: [f32; 4],
    pub emissive: [f32; 4],
    pub params: [f32; 4],
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: [0.8, 0.8, 0.8, 1.0],
            emissive: [0.0, 0.0, 0.0, 0.0],
            params: [0.0, 0.5, 0.0, 0.0],
        }
    }
}

#[derive(Clone, Debug)]
pub struct EyeGroup {
    pub nodes: Vec<usize>,
    pub gaze_nodes: Vec<usize>,
    pub pivot: Vec3,
}

#[derive(Clone, Debug)]
pub struct MeshAsset {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub nodes: Vec<Node>,
    pub materials: Vec<Material>,
    pub bounds_min: Vec3,
    pub bounds_max: Vec3,
    pub center: Vec3,
    pub size: Vec3,
    pub root: Option<usize>,
    pub body: Option<usize>,
    pub arm_l: Option<usize>,
    pub arm_r: Option<usize>,
    pub mouth: Option<usize>,
    pub spirit: Option<usize>,
    pub eyes: Vec<usize>,
    pub cheeks: Vec<usize>,
    pub eye_groups: Vec<EyeGroup>,
}

pub fn load_momo() -> Result<MeshAsset> {
    load_momo_from_slice(MOMO_GLB)
}

pub fn load_momo_from_slice(bytes: &[u8]) -> Result<MeshAsset> {
    let (document, buffers, _images) = gltf::import_slice(bytes)?;

    let gltf_nodes: Vec<_> = document.nodes().collect();
    let mut parent = vec![None; gltf_nodes.len()];
    for node in &gltf_nodes {
        for child in node.children() {
            parent[child.index()] = Some(node.index());
        }
    }

    let mut nodes = Vec::with_capacity(gltf_nodes.len());
    for node in &gltf_nodes {
        let name = node.name().unwrap_or("").to_string();
        let kind = node_kind(&name);
        let local = Mat4::from_cols_array_2d(&node.transform().matrix());
        nodes.push(Node {
            name,
            parent: parent[node.index()],
            local,
            rest_world: Mat4::IDENTITY,
            kind,
        });
    }

    let mut world_cache = vec![None; nodes.len()];
    for index in 0..nodes.len() {
        let world = world_matrix(index, &nodes, &mut world_cache);
        nodes[index].rest_world = world;
    }

    let mut materials: Vec<Material> = document
        .materials()
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let base = pbr.base_color_factor();
            let emissive = material.emissive_factor();
            let strength = material_strength(&material);
            let clearcoat = clearcoat_factor(&material);
            Material {
                base_color: base,
                emissive: [
                    emissive[0] * strength,
                    emissive[1] * strength,
                    emissive[2] * strength,
                    0.0,
                ],
                params: [pbr.metallic_factor(), pbr.roughness_factor(), clearcoat, 0.0],
            }
        })
        .collect();
    if materials.is_empty() {
        materials.push(Material::default());
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut bounds_min = Vec3::splat(f32::INFINITY);
    let mut bounds_max = Vec3::splat(f32::NEG_INFINITY);

    for node in &gltf_nodes {
        let Some(mesh) = node.mesh() else {
            continue;
        };
        let node_world = nodes[node.index()].rest_world;

        for primitive in mesh.primitives() {
            if primitive.mode() != Mode::Triangles {
                continue;
            }

            let reader = primitive.reader(|buffer| Some(buffers[buffer.index()].0.as_slice()));
            let Some(positions) = reader.read_positions() else {
                continue;
            };
            let positions: Vec<[f32; 3]> = positions.collect();
            if positions.is_empty() {
                continue;
            }

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|normals| normals.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            let material_id = primitive
                .material()
                .index()
                .filter(|index| *index < materials.len())
                .unwrap_or(0) as u32;

            let base = vertices.len() as u32;
            for (index, position) in positions.iter().enumerate() {
                let normal = normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]);
                vertices.push(Vertex {
                    position: *position,
                    normal,
                    node_id: node.index() as u32,
                    material_id,
                });

                let world_position =
                    node_world.transform_point3(Vec3::from_array(*position));
                bounds_min = bounds_min.min(world_position);
                bounds_max = bounds_max.max(world_position);
            }

            match reader.read_indices() {
                Some(index_data) => {
                    indices.extend(index_data.into_u32().map(|index| base + index));
                }
                None => {
                    indices.extend((0..positions.len() as u32).map(|index| base + index));
                }
            }
        }
    }

    if vertices.is_empty() {
        return Err("momo.glb contains no triangle geometry".into());
    }

    let center = (bounds_min + bounds_max) * 0.5;
    let size = bounds_max - bounds_min;

    let root = nodes
        .iter()
        .position(|node| node.name == "Momo_Root")
        .or_else(|| {
            nodes
                .iter()
                .position(|node| node.parent.is_none() && node.kind == NodeKind::Root)
        });

    let eye_groups = ["L", "R"]
        .into_iter()
        .filter_map(|side| {
            let suffix = format!("_{side}");
            let sclera = find_named(&nodes, &format!("Sclera{suffix}"))?;
            let pivot = nodes[sclera].rest_world.w_axis.truncate();
            let group_nodes: Vec<usize> = nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    (node.kind == NodeKind::Eye && node.name.ends_with(&suffix))
                        .then_some(index)
                })
                .collect();
            let gaze_nodes = group_nodes
                .iter()
                .copied()
                .filter(|&index| {
                    let name = &nodes[index].name;
                    name.starts_with("Iris_")
                        || name.starts_with("Pupil_")
                        || name.starts_with("HiBig_")
                        || name.starts_with("HiSmall_")
                })
                .collect();
            Some(EyeGroup {
                nodes: group_nodes,
                gaze_nodes,
                pivot,
            })
        })
        .collect();

    Ok(MeshAsset {
        vertices,
        indices,
        materials,
        bounds_min,
        bounds_max,
        center,
        size,
        root,
        body: find_named(&nodes, "Momo_Body"),
        arm_l: find_named(&nodes, "Arm_L"),
        arm_r: find_named(&nodes, "Arm_R"),
        mouth: find_named(&nodes, "Mouth"),
        spirit: find_named(&nodes, "Spirit"),
        eyes: nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (node.kind == NodeKind::Eye).then_some(index))
            .collect(),
        cheeks: nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (node.kind == NodeKind::Cheek).then_some(index))
            .collect(),
        eye_groups,
        nodes,
    })
}

fn find_named(nodes: &[Node], name: &str) -> Option<usize> {
    nodes.iter().position(|node| node.name == name)
}

fn node_kind(name: &str) -> NodeKind {
    if name == "Momo_Root" {
        NodeKind::Root
    } else if name == "Momo_Body" {
        NodeKind::Body
    } else if name.starts_with("Arm_") {
        NodeKind::Arm
    } else if name.starts_with("Cheek_") {
        NodeKind::Cheek
    } else if name.starts_with("Iris_")
        || name.starts_with("Pupil_")
        || name.starts_with("Sclera_")
        || name.starts_with("HiBig_")
        || name.starts_with("HiSmall_")
    {
        NodeKind::Eye
    } else if name == "Mouth" {
        NodeKind::Mouth
    } else if name == "Spirit" {
        NodeKind::Spirit
    } else {
        NodeKind::Other
    }
}

fn world_matrix(index: usize, nodes: &[Node], cache: &mut [Option<Mat4>]) -> Mat4 {
    if let Some(world) = cache[index] {
        return world;
    }

    let world = match nodes[index].parent {
        Some(parent) => world_matrix(parent, nodes, cache) * nodes[index].local,
        None => nodes[index].local,
    };
    cache[index] = Some(world);
    world
}

fn material_strength(material: &gltf::Material<'_>) -> f32 {
    material
        .extensions()
        .and_then(|extensions| extensions.get("KHR_materials_emissive_strength"))
        .and_then(|extension| extension.get("emissiveStrength"))
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
        .unwrap_or(1.0)
}

fn clearcoat_factor(material: &gltf::Material<'_>) -> f32 {
    material
        .extensions()
        .and_then(|extensions| extensions.get("KHR_materials_clearcoat"))
        .and_then(|extension| extension.get("clearcoatFactor"))
        .and_then(|value| value.as_f64())
        .map(|value| value as f32)
        .unwrap_or(0.0)
}

pub fn compute_worlds(nodes: &[Node], locals: &[Mat4]) -> Vec<Mat4> {
    let mut worlds: Vec<Option<Mat4>> = vec![None; nodes.len()];
    let mut pending: VecDeque<usize> = (0..nodes.len()).collect();

    while let Some(index) = pending.pop_front() {
        match nodes[index].parent {
            Some(parent) => {
                if let Some(parent_world) = worlds[parent] {
                    worlds[index] = Some(parent_world * locals[index]);
                } else {
                    pending.push_back(index);
                }
            }
            None => {
                worlds[index] = Some(locals[index]);
            }
        }
    }

    worlds
        .into_iter()
        .map(|world| world.unwrap_or(Mat4::IDENTITY))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_momo_geometry_and_hierarchy() {
        let asset = load_momo().expect("momo.glb should load");

        assert_eq!(asset.nodes.len(), 19);
        assert_eq!(asset.materials.len(), 8);
        assert_eq!(asset.vertices.len(), 24_319);
        assert_eq!(asset.indices.len(), 44_352 * 3);
        assert_eq!(asset.eyes.len(), 10);
        assert_eq!(asset.cheeks.len(), 2);
        assert_eq!(asset.eye_groups.len(), 2);
        assert!(asset.root.is_some());
        assert!(asset.arm_l.is_some());
        assert!(asset.arm_r.is_some());
        assert!(asset.mouth.is_some());
        assert!(asset.spirit.is_some());

        assert!((asset.size.x - 2.054).abs() < 0.01, "{:?}", asset.size);
        assert!((asset.size.y - 2.469).abs() < 0.01, "{:?}", asset.size);
        assert!((asset.center.y - 1.48).abs() < 0.01, "{:?}", asset.center);
    }

    #[test]
    fn world_transforms_place_body_above_origin() {
        let asset = load_momo().expect("momo.glb should load");
        let body = asset.body.expect("body node");
        let translation = asset.nodes[body].rest_world.w_axis.truncate();

        assert!((translation.y - 1.2).abs() < 0.001, "{translation:?}");
    }

    #[test]
    fn compute_worlds_handles_identity_parent() {
        let nodes = vec![
            Node {
                name: "root".to_string(),
                parent: None,
                local: Mat4::IDENTITY,
                rest_world: Mat4::IDENTITY,
                kind: NodeKind::Root,
            },
            Node {
                name: "child".to_string(),
                parent: Some(0),
                local: Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                rest_world: Mat4::IDENTITY,
                kind: NodeKind::Other,
            },
        ];
        let locals = [nodes[0].local, nodes[1].local];
        let worlds = compute_worlds(&nodes, &locals);

        assert_eq!(worlds[0], Mat4::IDENTITY);
        assert_eq!(worlds[1], Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn all_primitives_have_valid_material_ids() {
        let asset = load_momo().expect("momo.glb should load");
        assert!(asset
            .vertices
            .iter()
            .all(|vertex| (vertex.material_id as usize) < asset.materials.len()));
    }
}
