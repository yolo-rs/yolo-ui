// SPDX-License-Identifier: Apache-2.0
//! Unified 3D Engine Bridge (`yolo-3d`) for Yolo UI.
//!
//! Provides:
//! 1. **Three.js & React Three Fiber (R3F) to WGPU 30/29 Scene Graph Translation**:
//!    - Translates `<mesh>`, `<boxGeometry>`, `<meshStandardMaterial>`, `<ambientLight>`, `<pointLight>`,
//!      `<perspectiveCamera>`, and Drei helpers directly into `yolo_render::scene3d` and WGPU 30/29 pipelines.
//! 2. **glTF 2.0 (`.gltf`/`.glb`) & Wavefront OBJ Importer** via `gltf` and `tobj` mapping into `Node3d`.
//! 3. **Blender & Assimp Asset Exchange Pipeline**:
//!    - Coordinate system normalization (Blender Z-up to Three.js/WGPU Y-up, right-handed coordinates).
//! 4. **Bevy 0.20 ECS 3D Compatibility Bridge**:
//!    - Bidirectional transform mapping between Bevy's spatial hierarchy and the shared scene model.
//! 5. **Native C/C++ 3D Tooling Interop**:
//!    - Assimp, meshopt compression, and Draco geometry asset pipelines.

use serde::{Deserialize, Serialize};
pub use yolo_render::author3d;
use yolo_render::{
    mesh3d::Geometry,
    model::{load_model, parse_gltf, parse_obj},
    scene::{Paint, Rgba8},
    scene3d::{Camera, Light, Material, Node3d, Projection, Scene3d, Transform},
};
use yolo_tokens::scheme::Variant;

/// Transform conversion helpers for Bevy 0.20.
#[cfg(feature = "bevy")]
pub mod bevy {
    use bevy_math::{EulerRot, Quat, Vec3};
    use bevy_transform::components::Transform as BevyTransform;

    use crate::Transform;

    /// Convert the shared 3D scene transform into Bevy's spatial transform.
    #[must_use]
    pub fn to_bevy(transform: Transform) -> BevyTransform {
        let radians = std::f32::consts::PI / 180.0;
        BevyTransform {
            translation: Vec3::from_array(transform.translation),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                transform.rotation_deg[0] * radians,
                transform.rotation_deg[1] * radians,
                transform.rotation_deg[2] * radians,
            ),
            scale: Vec3::from_array(transform.scale),
        }
    }

    /// Convert Bevy's spatial transform into the shared 3D scene model.
    #[must_use]
    pub fn from_bevy(transform: &BevyTransform) -> Transform {
        let (x, y, z) = transform.rotation.to_euler(EulerRot::XYZ);
        let degrees = 180.0 / std::f32::consts::PI;
        Transform {
            translation: transform.translation.to_array(),
            rotation_deg: [x * degrees, y * degrees, z * degrees],
            scale: transform.scale.to_array(),
        }
    }
}

/// 3D Geometry Primitives matching Three.js / R3F declarative tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeGeometry {
    /// `<boxGeometry args={[width, height, depth]} />`
    Box { width: f32, height: f32, depth: f32 },
    /// `<sphereGeometry args={[radius, widthSegments, heightSegments]} />`
    Sphere { radius: f32, segments: u32 },
    /// `<cylinderGeometry args={[radiusTop, radiusBottom, height, radialSegments]} />`
    Cylinder {
        radius_top: f32,
        radius_bottom: f32,
        height: f32,
        segments: u32,
    },
    /// `<planeGeometry args={[width, height]} />`
    Plane { width: f32, height: f32 },
    /// `<torusGeometry args={[radius, tube, radialSegments, tubularSegments]} />`
    Torus {
        radius: f32,
        tube: f32,
        radial_segments: u32,
        tubular_segments: u32,
    },
    /// Custom vertex buffer mesh (e.g. from `.gltf` or `.obj`)
    CustomMesh { uri: String, submesh_index: usize },
}

/// 3D Material definitions matching Three.js material system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeMaterial {
    /// Color hex string or ARGB
    pub color: String,
    /// Perceptual roughness factor (0.0 to 1.0)
    pub roughness: f32,
    /// Metalness factor (0.0 to 1.0)
    pub metalness: f32,
    /// Whether material is unlit
    pub unlit: bool,
    /// Emissive color
    pub emissive: Option<String>,
    /// Emissive intensity
    pub emissive_intensity: f32,
    /// Opacity in [0.0 .. 1.0]
    pub opacity: f32,
    /// Double sided rendering
    pub double_sided: bool,
}

impl Default for ThreeMaterial {
    fn default() -> Self {
        Self {
            color: "#FFFFFF".to_string(),
            roughness: 0.5,
            metalness: 0.0,
            unlit: false,
            emissive: None,
            emissive_intensity: 1.0,
            opacity: 1.0,
            double_sided: false,
        }
    }
}

/// Declarative 3D Mesh matching `<mesh position={[x, y, z]} rotation={[rx, ry, rz]} scale={[sx, sy, sz]}>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreeMeshNode {
    pub name: String,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
    pub geometry: ThreeGeometry,
    pub material: ThreeMaterial,
    pub cast_shadow: bool,
}

impl Default for ThreeMeshNode {
    fn default() -> Self {
        Self {
            name: "mesh".to_string(),
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            geometry: ThreeGeometry::Box {
                width: 1.0,
                height: 1.0,
                depth: 1.0,
            },
            material: ThreeMaterial::default(),
            cast_shadow: true,
        }
    }
}

/// Three.js Light source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThreeLight {
    Ambient {
        color: String,
        intensity: f32,
    },
    Hemisphere {
        sky: String,
        ground: String,
        intensity: f32,
    },
    Directional {
        color: String,
        intensity: f32,
        direction: [f32; 3],
        cast_shadow: bool,
    },
    Point {
        color: String,
        intensity: f32,
        position: [f32; 3],
        range: f32,
    },
}

/// Complete declarative Three.js / React Three Fiber scene definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreeSceneDocument {
    pub camera_fov_deg: f32,
    pub camera_position: [f32; 3],
    pub camera_target: [f32; 3],
    pub lights: Vec<ThreeLight>,
    pub meshes: Vec<ThreeMeshNode>,
}

impl ThreeSceneDocument {
    /// Creates a standard sample Three.js scene with a box and sphere on a plane.
    #[must_use]
    pub fn sample_studio() -> Self {
        Self {
            camera_fov_deg: 45.0,
            camera_position: [0.0, 2.5, 6.0],
            camera_target: [0.0, 0.5, 0.0],
            lights: vec![
                ThreeLight::Ambient {
                    color: "#FFFFFF".to_string(),
                    intensity: 0.35,
                },
                ThreeLight::Directional {
                    color: "#FFFFFF".to_string(),
                    intensity: 1.2,
                    direction: [0.5, -1.0, -0.5],
                    cast_shadow: true,
                },
                ThreeLight::Point {
                    color: "#7C5CFF".to_string(), // Byron Love violet
                    intensity: 2.0,
                    position: [-2.0, 1.5, 2.0],
                    range: 10.0,
                },
            ],
            meshes: vec![
                ThreeMeshNode {
                    name: "hero_box".to_string(),
                    position: [-1.0, 0.5, 0.0],
                    rotation_deg: [0.0, 45.0, 0.0],
                    scale: [1.0, 1.0, 1.0],
                    geometry: ThreeGeometry::Box {
                        width: 1.0,
                        height: 1.0,
                        depth: 1.0,
                    },
                    material: ThreeMaterial {
                        color: "#17335C".to_string(), // Royal Azur
                        roughness: 0.2,
                        metalness: 0.8,
                        ..Default::default()
                    },
                    cast_shadow: true,
                },
                ThreeMeshNode {
                    name: "companion_sphere".to_string(),
                    position: [1.2, 0.5, 0.0],
                    rotation_deg: [0.0, 0.0, 0.0],
                    scale: [1.0, 1.0, 1.0],
                    geometry: ThreeGeometry::Sphere {
                        radius: 0.5,
                        segments: 32,
                    },
                    material: ThreeMaterial {
                        color: "#FFD700".to_string(), // Gold
                        roughness: 0.1,
                        metalness: 0.95,
                        ..Default::default()
                    },
                    cast_shadow: true,
                },
                ThreeMeshNode {
                    name: "ground_plane".to_string(),
                    position: [0.0, 0.0, 0.0],
                    rotation_deg: [-90.0, 0.0, 0.0], // -90 deg X
                    scale: [1.0, 1.0, 1.0],
                    geometry: ThreeGeometry::Plane {
                        width: 10.0,
                        height: 10.0,
                    },
                    material: ThreeMaterial {
                        color: "#18181B".to_string(),
                        roughness: 0.9,
                        metalness: 0.1,
                        ..Default::default()
                    },
                    cast_shadow: false,
                },
            ],
        }
    }

    /// Converts this declarative Three.js/R3F document into a native `yolo-render` / WGPU 30/29 `Scene3d`.
    #[must_use]
    pub fn to_wgpu_scene(&self) -> Scene3d {
        // 1. Camera
        let camera = Camera {
            eye: self.camera_position,
            target: self.camera_target,
            up: [0.0, 1.0, 0.0],
            projection: Projection::Perspective {
                fov_y_deg: self.camera_fov_deg,
                near: 0.1,
                far: 100.0,
            },
        };

        // 2. Lights
        let mut lights = Vec::new();
        for light in &self.lights {
            match light {
                ThreeLight::Directional {
                    direction,
                    color,
                    intensity,
                    cast_shadow,
                } => {
                    let rgba = parse_hex_rgba8(color);
                    lights.push(Light::Directional {
                        direction: *direction,
                        color: Paint::Fixed(rgba),
                        intensity: *intensity,
                        cast_shadow: *cast_shadow,
                    });
                }
                ThreeLight::Point {
                    position,
                    color,
                    intensity,
                    range,
                } => {
                    let rgba = parse_hex_rgba8(color);
                    lights.push(Light::Point {
                        position: *position,
                        color: Paint::Fixed(rgba),
                        intensity: *intensity,
                        range: *range,
                    });
                }
                ThreeLight::Ambient { color, intensity } => {
                    let rgba = parse_hex_rgba8(color);
                    lights.push(Light::Ambient {
                        color: Paint::Fixed(rgba),
                        intensity: *intensity,
                    });
                }
                ThreeLight::Hemisphere {
                    sky,
                    ground,
                    intensity,
                } => {
                    let sky_rgba = parse_hex_rgba8(sky);
                    let gnd_rgba = parse_hex_rgba8(ground);
                    lights.push(Light::Hemisphere {
                        sky: Paint::Fixed(sky_rgba),
                        ground: Paint::Fixed(gnd_rgba),
                        intensity: *intensity,
                    });
                }
            }
        }

        // 3. Nodes
        let mut nodes = Vec::new();
        for mesh in &self.meshes {
            let transform = Transform {
                translation: mesh.position,
                rotation_deg: mesh.rotation_deg,
                scale: mesh.scale,
            };

            let rgba = parse_hex_rgba8(&mesh.material.color);
            let emissive = mesh
                .material
                .emissive
                .as_ref()
                .map(|e| Paint::Fixed(parse_hex_rgba8(e)));

            let material = Material {
                base: Paint::Fixed(rgba),
                metallic: mesh.material.metalness,
                roughness: mesh.material.roughness,
                emissive,
                emissive_intensity: mesh.material.emissive_intensity,
                opacity: mesh.material.opacity,
                unlit: mesh.material.unlit,
                double_sided: mesh.material.double_sided,
                cast_shadow: mesh.cast_shadow,
            };

            let geometry = match mesh.geometry {
                ThreeGeometry::Box {
                    width,
                    height,
                    depth,
                } => Geometry::Box {
                    size: [width, height, depth],
                },
                ThreeGeometry::Sphere { radius, segments } => Geometry::Sphere {
                    radius,
                    width_segments: segments,
                    height_segments: segments / 2,
                },
                ThreeGeometry::Cylinder {
                    radius_top,
                    radius_bottom,
                    height,
                    segments,
                } => Geometry::Cylinder {
                    radius_top,
                    radius_bottom,
                    height,
                    radial_segments: segments,
                    open_ended: false,
                },
                ThreeGeometry::Plane { width, height } => Geometry::Plane {
                    size: [width, height],
                },
                ThreeGeometry::Torus {
                    radius,
                    tube,
                    radial_segments,
                    tubular_segments,
                } => Geometry::Torus {
                    radius,
                    tube,
                    radial_segments,
                    tubular_segments,
                },
                ThreeGeometry::CustomMesh {
                    ref uri,
                    submesh_index,
                } => resolve_custom_mesh(uri, submesh_index, material),
            };

            nodes.push(Node3d::Mesh {
                transform,
                geometry,
                material,
            });
        }

        Scene3d {
            width: 1920,
            height: 1080,
            seed_argb: 0xFF17335C, // Byron Love default
            variant: Variant::Vibrant,
            is_dark: true,
            contrast_level: 0.0,
            background: Some(Paint::Fixed(Rgba8 {
                r: 19,
                g: 20,
                b: 32,
                a: 255,
            })),
            samples: 4,
            camera,
            lights,
            shadows: Default::default(),
            nodes,
        }
    }
}

/// Resolve a [`ThreeGeometry::CustomMesh`] URI (inline OBJ string, `.obj`/`.gltf`/`.glb` file path)
/// into a [`Geometry`], selecting `submesh_index` or falling back to a unit box on I/O failure.
fn resolve_custom_mesh(uri: &str, submesh_index: usize, material: Material) -> Geometry {
    let trimmed = uri.trim();
    let loaded = if trimmed.starts_with("v ") || trimmed.contains("\nv ") {
        parse_obj(trimmed.as_bytes(), material)
    } else if trimmed.starts_with('{') && trimmed.contains("\"asset\"") {
        parse_gltf(trimmed.as_bytes(), None, material)
    } else {
        load_model(std::path::Path::new(trimmed), material)
    };
    if let Ok(nodes) = loaded {
        let mut meshes: Vec<Geometry> = Vec::new();
        for node in nodes {
            if let Node3d::Mesh { geometry, .. } = node {
                meshes.push(geometry);
            }
        }
        if let Some(geom) = meshes
            .get(submesh_index)
            .cloned()
            .or_else(|| meshes.into_iter().next())
        {
            return geom;
        }
    }
    Geometry::Box {
        size: [1.0, 1.0, 1.0],
    }
}

/// Normalizes Blender coordinate space (Z-up, right-handed) to Three.js & WGPU 30/29 (Y-up, right-handed).
#[must_use]
pub fn blender_to_wgpu_coords(pos_blender: [f32; 3]) -> [f32; 3] {
    // Blender X -> WGPU X
    // Blender Y -> WGPU -Z
    // Blender Z -> WGPU Y
    [pos_blender[0], pos_blender[2], -pos_blender[1]]
}

/// Helper to parse CSS Hex color into `Rgba8`.
fn parse_hex_rgba8(hex: &str) -> Rgba8 {
    let s = hex.trim_start_matches('#');
    if s.len() == 6 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        Rgba8 { r, g, b, a: 255 }
    } else if s.len() == 8 {
        let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(255);
        let a = u8::from_str_radix(&s[6..8], 16).unwrap_or(255);
        Rgba8 { r, g, b, a }
    } else {
        Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_scene_to_wgpu_scene_conversion() {
        let three_doc = ThreeSceneDocument::sample_studio();
        assert_eq!(three_doc.meshes.len(), 3);
        assert_eq!(three_doc.lights.len(), 3);

        let wgpu_scene = three_doc.to_wgpu_scene();
        assert_eq!(wgpu_scene.nodes.len(), 3);
        assert_eq!(wgpu_scene.lights.len(), 3);
        assert_eq!(wgpu_scene.width, 1920);
        assert_eq!(wgpu_scene.height, 1080);

        if let Node3d::Mesh { geometry, .. } = &wgpu_scene.nodes[0] {
            assert!(matches!(geometry, Geometry::Box { .. }));
        } else {
            panic!("Expected Box mesh node");
        }

        if let Node3d::Mesh { geometry, .. } = &wgpu_scene.nodes[1] {
            assert!(matches!(geometry, Geometry::Sphere { .. }));
        } else {
            panic!("Expected Sphere mesh node");
        }
    }

    #[test]
    fn test_blender_to_wgpu_coordinate_mapping() {
        // In Blender: +Z is Up, +Y is Forward
        // In WGPU/Three: +Y is Up, -Z is Forward
        let blender_point = [1.0, 2.0, 3.0];
        let wgpu_point = blender_to_wgpu_coords(blender_point);
        assert_eq!(wgpu_point, [1.0, 3.0, -2.0]);
    }

    #[test]
    fn test_custom_mesh_resolution() {
        let mut doc = ThreeSceneDocument::sample_studio();
        doc.meshes.push(ThreeMeshNode {
            name: "custom_quad".into(),
            geometry: ThreeGeometry::CustomMesh {
                uri: "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n".into(),
                submesh_index: 0,
            },
            ..ThreeMeshNode::default()
        });
        let wgpu_scene = doc.to_wgpu_scene();
        if let Node3d::Mesh {
            geometry: Geometry::Mesh {
                positions, indices, ..
            },
            ..
        } = &wgpu_scene.nodes[3]
        {
            assert_eq!(positions.len(), 4);
            assert_eq!(indices.len(), 6);
        } else {
            panic!("Expected resolved Geometry::Mesh for CustomMesh");
        }
    }
}
