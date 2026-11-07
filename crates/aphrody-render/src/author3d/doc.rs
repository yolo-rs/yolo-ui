// SPDX-License-Identifier: Apache-2.0
//! The 3D authoring document, its JSON command set and the bridge to the
//! renderer.
//!
//! A [`Document3d`] holds editable objects ([`PolyMesh`] + transform +
//! material + optional skin), a material library, one skeleton and animation
//! clips. Every edit is an [`Op`] (serde-tagged JSON, so CLIs, agents, MCP
//! tools and UIs drive the same surface); [`Editor`] applies ops atomically
//! with undo/redo. [`Document3d::scene`] evaluates the document at a time
//! (skinning + clip sampling) into the renderer's [`Scene3d`], which renders
//! on the CPU reference or `wgpu` backend and exports to GLB.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    anim::{Channel, Clip, Interpolation, Property},
    material::{MaterialDef, parse_paint, preset},
    poly::{PolyError, PolyMesh, Shading, Topology},
    rig::{RigError, Skeleton, SkinWeights, SkinningMethod, skin, skin_dual_quat, solve_fabrik},
    sculpt::{Brush, StrokeReport, apply_stroke},
    xform::{Quat, Trs, affine_inverse},
};
use crate::{
    math3d::{Vec3, srgb_to_linear},
    mesh3d::Geometry,
    model::{StudioOptions, studio_scene, write_glb},
    scene3d::{Node3d, Scene3d, Transform},
};

/// Upper bound on objects per document.
pub const MAX_OBJECTS: usize = 4096;
/// Upper bound on subdivision levels per operation.
pub const MAX_SUBDIVISION_LEVELS: u32 = 4;
/// Depth of the undo history.
pub const UNDO_LIMIT: usize = 64;

/// Why a document operation failed. A failed op leaves the document
/// unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc3dError {
    /// No object with that name.
    UnknownObject(String),
    /// An object with that name already exists.
    DuplicateObject(String),
    /// A modeling or sculpting operation failed.
    Mesh(PolyError),
    /// A rig or animation operation failed.
    Rig(RigError),
    /// A material is invalid or unknown.
    Material(String),
    /// Other invalid input.
    Invalid(String),
    /// Scene building, rendering or export failed.
    Render(String),
}

impl std::fmt::Display for Doc3dError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownObject(n) => write!(f, "unknown object {n:?}"),
            Self::DuplicateObject(n) => write!(f, "object {n:?} already exists"),
            Self::Mesh(e) => e.fmt(f),
            Self::Rig(e) => e.fmt(f),
            Self::Material(e) => write!(f, "material: {e}"),
            Self::Invalid(e) => write!(f, "invalid operation: {e}"),
            Self::Render(e) => write!(f, "render: {e}"),
        }
    }
}

impl std::error::Error for Doc3dError {}

impl From<PolyError> for Doc3dError {
    fn from(e: PolyError) -> Self {
        Self::Mesh(e)
    }
}

impl From<RigError> for Doc3dError {
    fn from(e: RigError) -> Self {
        Self::Rig(e)
    }
}

/// One editable object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Object3d {
    /// Unique name.
    pub name: String,
    /// Editable mesh in object space.
    pub mesh: PolyMesh,
    /// Object transform.
    #[serde(default)]
    pub transform: Trs,
    /// Material library key (`None` = default clay).
    #[serde(default)]
    pub material: Option<String>,
    /// Shading mode.
    #[serde(default)]
    pub shading: Shading,
    /// Skin binding to the document skeleton (weights per mesh vertex).
    #[serde(default)]
    pub skin: Option<SkinWeights>,
    /// Skinning evaluation method (`linear_blend` or `dual_quaternion`).
    #[serde(default)]
    pub skinning: SkinningMethod,
    /// Hidden objects are skipped when building scenes.
    #[serde(default = "yes")]
    pub visible: bool,
}

const fn yes() -> bool {
    true
}

/// The authoring document.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Document3d {
    /// Objects.
    #[serde(default)]
    pub objects: Vec<Object3d>,
    /// Material library (presets are available without definition).
    #[serde(default)]
    pub materials: BTreeMap<String, MaterialDef>,
    /// Skeleton shared by skinned objects (skeleton space = world space).
    #[serde(default)]
    pub skeleton: Skeleton,
    /// Animation clips.
    #[serde(default)]
    pub clips: Vec<Clip>,
}

/// Starting shape of a new object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum Primitive {
    /// Box of six quads.
    Cube {
        /// Extents.
        #[serde(default = "unit3")]
        size: [f32; 3],
    },
    /// Quad grid in XZ facing +Y.
    Grid {
        /// Extents.
        #[serde(default = "unit2")]
        size: [f32; 2],
        /// Quads along X and Z.
        #[serde(default = "grid_div")]
        divisions: [u32; 2],
    },
    /// All-quad sphere (subdivided cube).
    QuadSphere {
        /// Radius.
        #[serde(default = "half")]
        radius: f32,
        /// Subdivision level (0..=7).
        #[serde(default = "sphere_level")]
        level: u32,
    },
    /// Any renderer geometry, welded into connected topology.
    Geometry {
        /// Geometry.
        geometry: Geometry,
    },
}

const fn unit3() -> [f32; 3] {
    [1.0; 3]
}
const fn unit2() -> [f32; 2] {
    [1.0; 2]
}
const fn grid_div() -> [u32; 2] {
    [8, 8]
}
const fn half() -> f32 {
    0.5
}
const fn sphere_level() -> u32 {
    4
}

/// Euler-angle transform as written in commands.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TransformSpec {
    /// Translation.
    #[serde(default)]
    pub translation: [f32; 3],
    /// XYZ Euler rotation, degrees.
    #[serde(default)]
    pub rotation_euler_deg: [f32; 3],
    /// Scale.
    #[serde(default = "unit3")]
    pub scale: [f32; 3],
}

impl Default for TransformSpec {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation_euler_deg: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

impl TransformSpec {
    /// Convert to a quaternion transform.
    #[must_use]
    pub fn to_trs(&self) -> Trs {
        Trs {
            translation: self.translation,
            rotation: Quat::from_euler_xyz_deg(self.rotation_euler_deg),
            scale: self.scale,
        }
    }
}

/// Which faces an operator acts on.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "select", rename_all = "snake_case")]
pub enum FaceSelection {
    /// Every face.
    All,
    /// Explicit face indices.
    Indices {
        /// Face indices.
        faces: Vec<usize>,
    },
    /// Faces whose normal is within `max_angle_deg` of `direction`
    /// (object space).
    Facing {
        /// Direction.
        direction: [f32; 3],
        /// Cone half-angle, degrees.
        #[serde(default = "default_cone")]
        max_angle_deg: f32,
    },
}

const fn default_cone() -> f32 {
    1.0
}

impl FaceSelection {
    fn resolve(&self, mesh: &PolyMesh) -> Vec<usize> {
        match self {
            Self::All => (0..mesh.faces.len()).collect(),
            Self::Indices { faces } => faces.clone(),
            Self::Facing {
                direction,
                max_angle_deg,
            } => mesh.faces_facing(*direction, *max_angle_deg),
        }
    }
}

/// A ray in object space used to place sculpt dabs on the surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ray {
    /// Origin.
    pub origin: [f32; 3],
    /// Direction.
    pub direction: [f32; 3],
}

/// One key of a [`TrackSpec`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeySpec {
    /// Time, seconds.
    pub time: f32,
    /// Translation/scale `[x, y, z]`, or XYZ Euler degrees for rotation.
    pub value: [f32; 3],
}

/// One animated property of one bone, as written in commands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackSpec {
    /// Bone name.
    pub bone: String,
    /// Property.
    pub property: Property,
    /// Interpolation.
    #[serde(default)]
    pub interpolation: Interpolation,
    /// Keys in increasing time order.
    pub keys: Vec<KeySpec>,
}

/// A clip as written in commands.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipSpec {
    /// Name.
    pub name: String,
    /// Loop playback.
    #[serde(default)]
    pub looping: bool,
    /// Tracks.
    pub tracks: Vec<TrackSpec>,
}

impl ClipSpec {
    fn to_clip(&self) -> Clip {
        Clip {
            name: self.name.clone(),
            looping: self.looping,
            channels: self
                .tracks
                .iter()
                .map(|t| Channel {
                    bone: t.bone.clone(),
                    property: t.property,
                    interpolation: t.interpolation,
                    times: t.keys.iter().map(|k| k.time).collect(),
                    values: t
                        .keys
                        .iter()
                        .map(|k| match t.property {
                            Property::Rotation => Quat::from_euler_xyz_deg(k.value).into(),
                            _ => [k.value[0], k.value[1], k.value[2], 0.0],
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// A document edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Create an object.
    AddObject {
        /// Unique name.
        name: String,
        /// Starting shape.
        primitive: Primitive,
        /// Transform.
        #[serde(default)]
        transform: TransformSpec,
        /// Material key or preset.
        #[serde(default)]
        material: Option<String>,
        /// Shading.
        #[serde(default)]
        shading: Shading,
    },
    /// Delete an object.
    RemoveObject {
        /// Object.
        object: String,
    },
    /// Replace an object's transform.
    SetTransform {
        /// Object.
        object: String,
        /// Transform.
        transform: TransformSpec,
    },
    /// Region extrusion along averaged normals.
    Extrude {
        /// Object.
        object: String,
        /// Faces.
        faces: FaceSelection,
        /// Distance (negative pushes inwards).
        distance: f32,
    },
    /// Per-face inset.
    Inset {
        /// Object.
        object: String,
        /// Faces.
        faces: FaceSelection,
        /// Fraction towards the centroid, `(0, 1)`.
        amount: f32,
    },
    /// Catmull-Clark subdivision.
    Subdivide {
        /// Object.
        object: String,
        /// Levels (1..=4).
        #[serde(default = "one_level")]
        levels: u32,
    },
    /// Weld close vertices.
    MergeByDistance {
        /// Object.
        object: String,
        /// Distance.
        distance: f32,
    },
    /// One sculpt stroke (object space).
    Sculpt {
        /// Object.
        object: String,
        /// Brush.
        brush: Brush,
        /// Stroke points on the surface.
        #[serde(default)]
        stroke: Vec<[f32; 3]>,
        /// Rays picked against the surface; hits are appended to `stroke`.
        #[serde(default)]
        rays: Vec<Ray>,
    },
    /// Add or replace a library material.
    DefineMaterial {
        /// Key.
        name: String,
        /// Definition.
        material: MaterialDef,
    },
    /// Assign a library material or preset.
    AssignMaterial {
        /// Object.
        object: String,
        /// Key or preset name.
        material: String,
    },
    /// Change shading.
    SetShading {
        /// Object.
        object: String,
        /// Shading.
        shading: Shading,
    },
    /// Append a bone to the skeleton.
    AddBone {
        /// Unique name.
        name: String,
        /// Parent bone.
        #[serde(default)]
        parent: Option<String>,
        /// Local rest transform relative to the parent.
        #[serde(default)]
        transform: TransformSpec,
        /// Display/leaf length.
        #[serde(default = "bone_length")]
        length: f32,
    },
    /// Compute automatic weights and bind the object to the skeleton.
    BindSkin {
        /// Object.
        object: String,
        /// Falloff sharpness.
        #[serde(default = "weight_power")]
        power: f32,
    },
    /// Add or replace an animation clip.
    AddClip {
        /// Clip.
        clip: ClipSpec,
    },
    /// Bevel (chamfer) edges of an object (`edges` empty = all edges).
    Bevel {
        /// Object.
        object: String,
        /// Optional undirected edges `[u, v]`.
        #[serde(default)]
        edges: Vec<[u32; 2]>,
        /// Fractional bevel offset `(0.0, 0.5)`.
        amount: f32,
    },
    /// Insert an edge loop across the quad ring starting at `(face, edge)`.
    LoopCut {
        /// Object.
        object: String,
        /// Starting quad face index.
        face: usize,
        /// Local edge index `0..4`.
        edge: usize,
        /// Cut factor `(0.0, 1.0)`.
        #[serde(default = "half")]
        factor: f32,
    },
    /// Bridge two faces of equal vertex count with a quad tube.
    Bridge {
        /// Object.
        object: String,
        /// First cap face index.
        face_a: usize,
        /// Second cap face index.
        face_b: usize,
    },
    /// Bind an object to the skeleton using surface-graph geodesic auto-weights.
    BindSkinGeodesic {
        /// Object.
        object: String,
        /// Falloff sharpness.
        #[serde(default = "weight_power")]
        power: f32,
    },
    /// Switch an object between Linear Blend Skinning and Dual Quaternion Skinning.
    SetSkinningMethod {
        /// Object.
        object: String,
        /// Skinning algorithm.
        method: SkinningMethod,
    },
    /// Solve FABRIK inverse kinematics on a bone chain and write the solved
    /// rest pose into the skeleton's local transforms.
    SolveIk {
        /// Ordered parent-to-child bone names.
        chain: Vec<String>,
        /// Target position in world/skeleton space.
        target: [f32; 3],
        /// Convergence tolerance.
        #[serde(default = "ik_tol")]
        tolerance: f32,
        /// Maximum solver iterations.
        #[serde(default = "ik_iters")]
        max_iterations: u32,
    },
}

const fn one_level() -> u32 {
    1
}
const fn bone_length() -> f32 {
    0.25
}
const fn weight_power() -> f32 {
    4.0
}
const fn ik_tol() -> f32 {
    1e-4
}
const fn ik_iters() -> u32 {
    24
}

/// What an operation did.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct OpReport {
    /// Object touched, when any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Topology after the op.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology: Option<Topology>,
    /// Faces created or selected by the op.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub faces: Vec<usize>,
    /// Sculpt result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<StrokeReport>,
    /// A topology change invalidated the object's skin binding.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub skin_cleared: bool,
}

/// How to evaluate and frame the document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewSpec {
    /// Width, pixels.
    pub width: u32,
    /// Height, pixels.
    pub height: u32,
    /// Clip to play (`None` = rest pose).
    pub clip: Option<String>,
    /// Time, seconds.
    pub time: f32,
    /// Camera azimuth, degrees.
    pub yaw_deg: f32,
    /// Camera elevation, degrees.
    pub pitch_deg: f32,
    /// Zoom (1 = framed).
    pub zoom: f32,
    /// M3 seed colour `0xAARRGGBB`.
    pub seed_argb: u32,
    /// Dark scheme.
    pub is_dark: bool,
    /// Background paint (hex or M3 role; `None` = transparent).
    pub background: Option<String>,
    /// Shadow mapping.
    pub shadows: bool,
}

impl Default for ViewSpec {
    fn default() -> Self {
        let s = StudioOptions::default();
        Self {
            width: 640,
            height: 480,
            clip: None,
            time: 0.0,
            yaw_deg: s.yaw_deg,
            pitch_deg: s.pitch_deg,
            zoom: s.zoom,
            seed_argb: s.seed_argb,
            is_dark: false,
            background: Some("surface_container".into()),
            shadows: true,
        }
    }
}

/// Per-object summary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObjectStats {
    /// Name.
    pub name: String,
    /// Topology.
    pub topology: Topology,
    /// Signed volume.
    pub volume: f32,
    /// Bound to the skeleton.
    pub skinned: bool,
    /// Material key.
    pub material: Option<String>,
}

/// Document summary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DocumentStats {
    /// Objects.
    pub objects: Vec<ObjectStats>,
    /// Bone names.
    pub bones: Vec<String>,
    /// Clip names and durations.
    pub clips: Vec<(String, f32)>,
}

impl Document3d {
    fn index(&self, name: &str) -> Result<usize, Doc3dError> {
        self.objects
            .iter()
            .position(|o| o.name == name)
            .ok_or_else(|| Doc3dError::UnknownObject(name.to_string()))
    }

    /// Object by name.
    #[must_use]
    pub fn object(&self, name: &str) -> Option<&Object3d> {
        self.objects.iter().find(|o| o.name == name)
    }

    /// Resolve a material key: library first, then presets.
    ///
    /// # Errors
    /// [`Doc3dError::Material`] for unknown keys or invalid definitions.
    pub fn material_def(&self, key: &str) -> Result<MaterialDef, Doc3dError> {
        self.materials
            .get(key)
            .cloned()
            .or_else(|| preset(key))
            .ok_or_else(|| Doc3dError::Material(format!("unknown material {key:?}")))
    }

    fn check_material(&self, key: &str) -> Result<(), Doc3dError> {
        self.material_def(key)?
            .to_material()
            .map(|_| ())
            .map_err(Doc3dError::Material)
    }

    /// Apply one operation in place. Prefer [`Editor::apply`], which is
    /// atomic and undoable; this mutates directly and may leave partial
    /// changes on error.
    ///
    /// # Errors
    /// [`Doc3dError`] describing why the op was refused.
    pub fn apply(&mut self, op: &Op) -> Result<OpReport, Doc3dError> {
        let mut report = OpReport::default();
        match op {
            Op::AddObject {
                name,
                primitive,
                transform,
                material,
                shading,
            } => {
                if name.is_empty() {
                    return Err(Doc3dError::Invalid("object name is empty".into()));
                }
                if self.object(name).is_some() {
                    return Err(Doc3dError::DuplicateObject(name.clone()));
                }
                if self.objects.len() >= MAX_OBJECTS {
                    return Err(Doc3dError::Invalid("too many objects".into()));
                }
                if let Some(m) = material {
                    self.check_material(m)?;
                }
                let mesh = match primitive {
                    Primitive::Cube { size } => {
                        if !size.iter().all(|v| *v > 0.0 && v.is_finite()) {
                            return Err(Doc3dError::Invalid("cube size must be positive".into()));
                        }
                        PolyMesh::cube(*size)
                    }
                    Primitive::Grid { size, divisions } => PolyMesh::grid(*size, *divisions)?,
                    Primitive::QuadSphere { radius, level } => {
                        PolyMesh::quad_sphere(*radius, *level)?
                    }
                    Primitive::Geometry { geometry } => PolyMesh::from_geometry(geometry)?,
                };
                let trs = transform.to_trs();
                if !trs.is_finite() {
                    return Err(Doc3dError::Invalid("transform is not finite".into()));
                }
                report.topology = Some(mesh.topology());
                report.object = Some(name.clone());
                self.objects.push(Object3d {
                    name: name.clone(),
                    mesh,
                    transform: trs,
                    material: material.clone(),
                    shading: *shading,
                    skin: None,
                    skinning: SkinningMethod::default(),
                    visible: true,
                });
            }
            Op::RemoveObject { object } => {
                let i = self.index(object)?;
                self.objects.remove(i);
                report.object = Some(object.clone());
            }
            Op::SetTransform { object, transform } => {
                let i = self.index(object)?;
                let trs = transform.to_trs();
                if !trs.is_finite() {
                    return Err(Doc3dError::Invalid("transform is not finite".into()));
                }
                self.objects[i].transform = trs;
                report.object = Some(object.clone());
            }
            Op::Extrude {
                object,
                faces,
                distance,
            } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                let sel = faces.resolve(&o.mesh);
                report.faces = o.mesh.extrude_region(&sel, *distance)?;
                report.skin_cleared = o.skin.take().is_some();
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::Inset {
                object,
                faces,
                amount,
            } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                let sel = faces.resolve(&o.mesh);
                report.faces = o.mesh.inset_faces(&sel, *amount)?;
                report.skin_cleared = o.skin.take().is_some();
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::Subdivide { object, levels } => {
                if *levels == 0 || *levels > MAX_SUBDIVISION_LEVELS {
                    return Err(Doc3dError::Invalid(format!(
                        "subdivision levels must be 1..={MAX_SUBDIVISION_LEVELS}"
                    )));
                }
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                for _ in 0..*levels {
                    o.mesh = o.mesh.catmull_clark()?;
                }
                report.skin_cleared = o.skin.take().is_some();
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::MergeByDistance { object, distance } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                if o.mesh.merge_by_distance(*distance) > 0 {
                    report.skin_cleared = o.skin.take().is_some();
                }
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::Sculpt {
                object,
                brush,
                stroke,
                rays,
            } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                let mut points = stroke.clone();
                for r in rays {
                    if let Some(hit) = o.mesh.raycast(r.origin, r.direction) {
                        points.push(hit.point);
                    }
                }
                if points.is_empty() {
                    return Err(Doc3dError::Invalid(
                        "sculpt stroke is empty (no points and no ray hit the surface)".into(),
                    ));
                }
                report.stroke = Some(apply_stroke(&mut o.mesh, brush, &points, None)?);
                report.object = Some(object.clone());
            }
            Op::DefineMaterial { name, material } => {
                if name.is_empty() {
                    return Err(Doc3dError::Material("material name is empty".into()));
                }
                material.to_material().map_err(Doc3dError::Material)?;
                self.materials.insert(name.clone(), material.clone());
            }
            Op::AssignMaterial { object, material } => {
                self.check_material(material)?;
                let i = self.index(object)?;
                self.objects[i].material = Some(material.clone());
                report.object = Some(object.clone());
            }
            Op::SetShading { object, shading } => {
                let i = self.index(object)?;
                self.objects[i].shading = *shading;
                report.object = Some(object.clone());
            }
            Op::AddBone {
                name,
                parent,
                transform,
                length,
            } => {
                self.skeleton
                    .add_bone(name, parent.as_deref(), transform.to_trs(), *length)?;
                // Existing bindings index the old bone set; rebinding is
                // required, so drop them rather than deform wrongly.
                for o in &mut self.objects {
                    report.skin_cleared |= o.skin.take().is_some();
                }
            }
            Op::BindSkin { object, power } => {
                let i = self.index(object)?;
                let o = &self.objects[i];
                let m = o.transform.matrix();
                let world: Vec<[f32; 3]> = o
                    .mesh
                    .positions
                    .iter()
                    .map(|p| m.transform_point(Vec3::from_array(*p)).to_array())
                    .collect();
                let weights = self.skeleton.auto_weights(&world, *power)?;
                self.objects[i].skin = Some(weights);
                report.object = Some(object.clone());
            }
            Op::AddClip { clip } => {
                let c = clip.to_clip();
                c.validate(&self.skeleton)?;
                self.clips.retain(|x| x.name != c.name);
                self.clips.push(c);
            }
            Op::Bevel {
                object,
                edges,
                amount,
            } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                report.faces = o.mesh.bevel_edges(edges, *amount)?;
                report.skin_cleared = o.skin.take().is_some();
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::LoopCut {
                object,
                face,
                edge,
                factor,
            } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                o.mesh.loop_cut(*face, *edge, *factor)?;
                report.skin_cleared = o.skin.take().is_some();
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::Bridge {
                object,
                face_a,
                face_b,
            } => {
                let i = self.index(object)?;
                let o = &mut self.objects[i];
                report.faces = o.mesh.bridge_faces(*face_a, *face_b)?;
                report.skin_cleared = o.skin.take().is_some();
                report.topology = Some(o.mesh.topology());
                report.object = Some(object.clone());
            }
            Op::BindSkinGeodesic { object, power } => {
                let i = self.index(object)?;
                let o = &self.objects[i];
                let m = o.transform.matrix();
                let world: Vec<[f32; 3]> = o
                    .mesh
                    .positions
                    .iter()
                    .map(|p| m.transform_point(Vec3::from_array(*p)).to_array())
                    .collect();
                let weights =
                    self.skeleton
                        .auto_weights_geodesic(&world, &o.mesh.faces, *power)?;
                self.objects[i].skin = Some(weights);
                report.object = Some(object.clone());
            }
            Op::SetSkinningMethod { object, method } => {
                let i = self.index(object)?;
                self.objects[i].skinning = *method;
                report.object = Some(object.clone());
            }
            Op::SolveIk {
                chain,
                target,
                tolerance,
                max_iterations,
            } => {
                let mut indices = Vec::with_capacity(chain.len());
                for name in chain {
                    let bi = self
                        .skeleton
                        .find(name)
                        .ok_or_else(|| Doc3dError::Invalid(format!("unknown bone {name:?}")))?;
                    indices.push(bi);
                }
                let mut pose = self.skeleton.rest_pose();
                solve_fabrik(
                    &self.skeleton,
                    &mut pose,
                    &indices,
                    *target,
                    *tolerance,
                    *max_iterations,
                )?;
                for (b, local) in self.skeleton.bones.iter_mut().zip(pose.locals) {
                    b.rest = local;
                }
            }
        }
        Ok(report)
    }

    /// Evaluate the document into renderer nodes (world space, posed).
    ///
    /// # Errors
    /// [`Doc3dError`] for unknown clips, bad materials or rig failures.
    pub fn nodes(&self, clip: Option<&str>, time: f32) -> Result<Vec<Node3d>, Doc3dError> {
        let pose = match clip {
            Some(name) => {
                let c = self
                    .clips
                    .iter()
                    .find(|c| c.name == name)
                    .ok_or_else(|| Doc3dError::Invalid(format!("unknown clip {name:?}")))?;
                Some(c.sample(&self.skeleton, time)?)
            }
            None => None,
        };
        let matrices = match &pose {
            Some(p) => Some(self.skeleton.skin_matrices(p)?),
            None => None,
        };
        let mut nodes = Vec::new();
        for o in self.objects.iter().filter(|o| o.visible) {
            let m = o.transform.matrix();
            if affine_inverse(&m).is_none() {
                return Err(Doc3dError::Invalid(format!(
                    "object {:?} has a singular transform",
                    o.name
                )));
            }
            let mut world: Vec<[f32; 3]> = o
                .mesh
                .positions
                .iter()
                .map(|p| m.transform_point(Vec3::from_array(*p)).to_array())
                .collect();
            if let (Some(w), Some(mats)) = (&o.skin, &matrices) {
                world = match o.skinning {
                    SkinningMethod::LinearBlend => skin(&world, None, w, mats)?.0,
                    SkinningMethod::DualQuaternion => skin_dual_quat(&world, None, w, mats)?.0,
                };
            }
            let baked = PolyMesh {
                positions: world,
                faces: o.mesh.faces.clone(),
            };
            let material = match &o.material {
                Some(key) => self.material_def(key)?,
                None => MaterialDef::default(),
            }
            .to_material()
            .map_err(Doc3dError::Material)?;
            nodes.push(Node3d::Mesh {
                transform: Transform::IDENTITY,
                geometry: baked.to_geometry(o.shading),
                material,
            });
        }
        Ok(nodes)
    }

    /// Stage the evaluated document in the M3 studio (lights + framed
    /// camera).
    ///
    /// # Errors
    /// [`Doc3dError`] for evaluation failures or an empty document.
    pub fn scene(&self, view: &ViewSpec) -> Result<Scene3d, Doc3dError> {
        let nodes = self.nodes(view.clip.as_deref(), view.time)?;
        let background = view
            .background
            .as_deref()
            .map(parse_paint)
            .transpose()
            .map_err(Doc3dError::Invalid)?;
        let options = StudioOptions {
            seed_argb: view.seed_argb,
            is_dark: view.is_dark,
            background,
            yaw_deg: view.yaw_deg,
            pitch_deg: view.pitch_deg,
            zoom: view.zoom,
            shadows: view.shadows,
            ..StudioOptions::default()
        };
        // Frame the camera on the rest pose so animated frames share one
        // camera instead of re-framing (and jittering) every frame.
        let framing = if view.clip.is_some() {
            self.nodes(None, 0.0)?
        } else {
            nodes.clone()
        };
        let mut scene = studio_scene(framing, view.width, view.height, &options)
            .map_err(|e| Doc3dError::Render(e.to_string()))?;
        scene.nodes = nodes;
        Ok(scene)
    }

    /// Render a view on the deterministic CPU reference backend as PNG.
    ///
    /// # Errors
    /// [`Doc3dError`] for evaluation or render failures.
    pub fn render_png(&self, view: &ViewSpec) -> Result<Vec<u8>, Doc3dError> {
        crate::raster3d::render_scene3d_png(&self.scene(view)?)
            .map_err(|e| Doc3dError::Render(e.to_string()))
    }

    /// Export the evaluated (posed, world-space) document as binary glTF.
    ///
    /// # Errors
    /// [`Doc3dError`] for evaluation or export failures.
    pub fn export_glb(&self, view: &ViewSpec) -> Result<Vec<u8>, Doc3dError> {
        let scene = self.scene(view)?;
        write_glb(&scene.nodes, &scene.scheme()).map_err(|e| Doc3dError::Render(e.to_string()))
    }

    /// Export the document as an unbaked, rigged binary glTF 2.0 (`.glb`)
    /// preserving the skeleton node hierarchy, `skins` (`inverseBindMatrices`
    /// + `JOINTS_0`/`WEIGHTS_0`), and `animations` (`samplers` + `channels`).
    ///
    /// # Errors
    /// [`Doc3dError`] for evaluation or export failures.
    pub fn export_skinned_glb(&self, view: &ViewSpec) -> Result<Vec<u8>, Doc3dError> {
        use aphrody_m3_tokens::facade::color::{DynamicScheme, Variant};
        use serde_json::json;

        let scheme = DynamicScheme::from_argb(view.seed_argb, Variant::TonalSpot, view.is_dark, 0.0);
        let mut bin: Vec<u8> = Vec::new();
        let mut views = Vec::new();
        let mut accessors = Vec::new();
        let mut gnodes = Vec::new();
        let mut meshes = Vec::new();
        let mut materials = Vec::new();
        let mut skins = Vec::new();
        let mut animations = Vec::new();
        let mut root_nodes = Vec::new();

        let push_view = |bin: &mut Vec<u8>, views: &mut Vec<serde_json::Value>, bytes: &[u8], target: Option<u32>| -> usize {
            while bin.len() % 4 != 0 {
                bin.push(0);
            }
            let offset = bin.len();
            bin.extend_from_slice(bytes);
            let mut v = json!({
                "buffer": 0,
                "byteOffset": offset,
                "byteLength": bytes.len(),
            });
            if let Some(t) = target {
                v["target"] = json!(t);
            }
            views.push(v);
            views.len() - 1
        };

        let bone_count = self.skeleton.bones.len();
        for (i, bone) in self.skeleton.bones.iter().enumerate() {
            let children: Vec<usize> = (0..bone_count)
                .filter(|&c| self.skeleton.bones[c].parent == Some(i))
                .collect();
            let mut node = json!({
                "name": bone.name,
                "translation": bone.rest.translation,
                "rotation": [bone.rest.rotation.x, bone.rest.rotation.y, bone.rest.rotation.z, bone.rest.rotation.w],
                "scale": bone.rest.scale,
            });
            if !children.is_empty() {
                node["children"] = json!(children);
            }
            if bone.parent.is_none() {
                root_nodes.push(i);
            }
            gnodes.push(node);
        }

        if bone_count > 0 {
            let ibms = self.skeleton.inverse_bind()?;
            let ibm_bytes: Vec<u8> = ibms
                .iter()
                .flat_map(|m| m.cols.iter().flatten())
                .flat_map(|f| f.to_le_bytes())
                .collect();
            let ibm_view = push_view(&mut bin, &mut views, &ibm_bytes, None);
            accessors.push(json!({
                "bufferView": ibm_view,
                "componentType": 5126,
                "count": bone_count,
                "type": "MAT4",
            }));
            let ibm_acc = accessors.len() - 1;
            skins.push(json!({
                "name": "skeleton",
                "inverseBindMatrices": ibm_acc,
                "joints": (0..bone_count).collect::<Vec<_>>(),
            }));
        }

        for o in self.objects.iter().filter(|o| o.visible) {
            let m = o.transform.matrix();
            if affine_inverse(&m).is_none() {
                return Err(Doc3dError::Invalid(format!(
                    "object {:?} has a singular transform",
                    o.name
                )));
            }
            let world_positions: Vec<[f32; 3]> = o
                .mesh
                .positions
                .iter()
                .map(|p| m.transform_point(Vec3::from_array(*p)).to_array())
                .collect();
            let baked = PolyMesh {
                positions: world_positions,
                faces: o.mesh.faces.clone(),
            };
            let (tris, normals) = baked.triangulate();
            let indices: Vec<u32> = tris
                .iter()
                .flat_map(|t| [t[0] as u32, t[1] as u32, t[2] as u32])
                .collect();

            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in &baked.positions {
                for axis in 0..3 {
                    lo[axis] = lo[axis].min(p[axis]);
                    hi[axis] = hi[axis].max(p[axis]);
                }
            }
            let pos_bytes: Vec<u8> = baked
                .positions
                .iter()
                .flatten()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            let pos_view = push_view(&mut bin, &mut views, &pos_bytes, Some(34962));
            accessors.push(json!({
                "bufferView": pos_view,
                "componentType": 5126,
                "count": baked.positions.len(),
                "type": "VEC3",
                "min": lo,
                "max": hi,
            }));
            let pos_acc = accessors.len() - 1;

            let nrm_bytes: Vec<u8> = normals
                .iter()
                .flatten()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            let nrm_view = push_view(&mut bin, &mut views, &nrm_bytes, Some(34962));
            accessors.push(json!({
                "bufferView": nrm_view,
                "componentType": 5126,
                "count": normals.len(),
                "type": "VEC3",
            }));
            let nrm_acc = accessors.len() - 1;

            let idx_bytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
            let idx_view = push_view(&mut bin, &mut views, &idx_bytes, Some(34963));
            accessors.push(json!({
                "bufferView": idx_view,
                "componentType": 5125,
                "count": indices.len(),
                "type": "SCALAR",
            }));
            let idx_acc = accessors.len() - 1;

            let mat_def = match &o.material {
                Some(key) => self.material_def(key)?,
                None => MaterialDef::default(),
            };
            let mat = mat_def.to_material().map_err(Doc3dError::Material)?;
            let base = mat.base.resolve(&scheme);
            let lin = |c: u8| srgb_to_linear(f32::from(c) / 255.0);
            let alpha = f32::from(base.a) / 255.0 * mat.opacity.clamp(0.0, 1.0);
            materials.push(json!({
                "pbrMetallicRoughness": {
                    "baseColorFactor": [lin(base.r), lin(base.g), lin(base.b), alpha],
                    "metallicFactor": mat.metallic.clamp(0.0, 1.0),
                    "roughnessFactor": mat.roughness.clamp(0.0, 1.0),
                },
                "doubleSided": mat.double_sided,
            }));
            let mat_idx = materials.len() - 1;

            let mut attrs = json!({
                "POSITION": pos_acc,
                "NORMAL": nrm_acc,
            });
            let mut is_skinned = false;
            if let Some(w) = &o.skin {
                if bone_count > 0 && w.joints.len() == baked.positions.len() {
                    let j_bytes: Vec<u8> = w
                        .joints
                        .iter()
                        .flatten()
                        .flat_map(|j| j.to_le_bytes())
                        .collect();
                    let j_view = push_view(&mut bin, &mut views, &j_bytes, Some(34962));
                    accessors.push(json!({
                        "bufferView": j_view,
                        "componentType": 5123,
                        "count": w.joints.len(),
                        "type": "VEC4",
                    }));
                    let j_acc = accessors.len() - 1;

                    let w_bytes: Vec<u8> = w
                        .weights
                        .iter()
                        .flatten()
                        .flat_map(|f| f.to_le_bytes())
                        .collect();
                    let w_view = push_view(&mut bin, &mut views, &w_bytes, Some(34962));
                    accessors.push(json!({
                        "bufferView": w_view,
                        "componentType": 5126,
                        "count": w.weights.len(),
                        "type": "VEC4",
                    }));
                    let w_acc = accessors.len() - 1;

                    attrs["JOINTS_0"] = json!(j_acc);
                    attrs["WEIGHTS_0"] = json!(w_acc);
                    is_skinned = true;
                }
            }

            meshes.push(json!({
                "name": o.name,
                "primitives": [{
                    "attributes": attrs,
                    "indices": idx_acc,
                    "material": mat_idx,
                }],
            }));
            let mesh_idx = meshes.len() - 1;
            let mut gnode = json!({
                "name": o.name,
                "mesh": mesh_idx,
            });
            if is_skinned {
                gnode["skin"] = json!(0);
            }
            gnodes.push(gnode);
            root_nodes.push(gnodes.len() - 1);
        }

        if meshes.is_empty() {
            return Err(Doc3dError::Render("nothing to export".into()));
        }

        for clip in &self.clips {
            let mut samplers = Vec::new();
            let mut channels = Vec::new();
            for ch in &clip.channels {
                let Some(bi) = self.skeleton.find(&ch.bone) else {
                    continue;
                };
                if ch.times.is_empty() {
                    continue;
                }
                let t_min = ch.times.iter().copied().fold(f32::MAX, f32::min);
                let t_max = ch.times.iter().copied().fold(f32::MIN, f32::max);
                let t_bytes: Vec<u8> = ch.times.iter().flat_map(|t| t.to_le_bytes()).collect();
                let t_view = push_view(&mut bin, &mut views, &t_bytes, None);
                accessors.push(json!({
                    "bufferView": t_view,
                    "componentType": 5126,
                    "count": ch.times.len(),
                    "type": "SCALAR",
                    "min": [t_min],
                    "max": [t_max],
                }));
                let t_acc = accessors.len() - 1;

                let (val_bytes, acc_type, path): (Vec<u8>, &str, &str) = match ch.property {
                    Property::Translation => (
                        ch.values
                            .iter()
                            .flat_map(|v| [v[0], v[1], v[2]])
                            .flat_map(|f| f.to_le_bytes())
                            .collect(),
                        "VEC3",
                        "translation",
                    ),
                    Property::Rotation => (
                        ch.values
                            .iter()
                            .flatten()
                            .flat_map(|f| f.to_le_bytes())
                            .collect(),
                        "VEC4",
                        "rotation",
                    ),
                    Property::Scale => (
                        ch.values
                            .iter()
                            .flat_map(|v| [v[0], v[1], v[2]])
                            .flat_map(|f| f.to_le_bytes())
                            .collect(),
                        "VEC3",
                        "scale",
                    ),
                };
                let v_view = push_view(&mut bin, &mut views, &val_bytes, None);
                accessors.push(json!({
                    "bufferView": v_view,
                    "componentType": 5126,
                    "count": ch.values.len(),
                    "type": acc_type,
                }));
                let v_acc = accessors.len() - 1;

                let interp = match ch.interpolation {
                    Interpolation::Step => "STEP",
                    Interpolation::Linear | Interpolation::Smooth => "LINEAR",
                };
                samplers.push(json!({
                    "input": t_acc,
                    "interpolation": interp,
                    "output": v_acc,
                }));
                let sampler_idx = samplers.len() - 1;
                channels.push(json!({
                    "sampler": sampler_idx,
                    "target": {
                        "node": bi,
                        "path": path,
                    },
                }));
            }
            if !channels.is_empty() {
                animations.push(json!({
                    "name": clip.name,
                    "samplers": samplers,
                    "channels": channels,
                }));
            }
        }

        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let mut doc = json!({
            "asset": { "version": "2.0", "generator": "aphrody-render" },
            "scene": 0,
            "scenes": [{ "nodes": root_nodes }],
            "nodes": gnodes,
            "meshes": meshes,
            "materials": materials,
            "accessors": accessors,
            "bufferViews": views,
            "buffers": [{ "byteLength": bin.len() }],
        });
        if !skins.is_empty() {
            doc["skins"] = json!(skins);
        }
        if !animations.is_empty() {
            doc["animations"] = json!(animations);
        }

        let mut json_bytes =
            serde_json::to_vec(&doc).map_err(|e| Doc3dError::Render(e.to_string()))?;
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }
        let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2_u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json_bytes);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
        Ok(out)
    }

    /// Import a glTF 2.0 (`.gltf` or `.glb`) byte buffer into an editable
    /// [`Document3d`], reconstructing meshes, skeleton joints, skin weights
    /// (`JOINTS_0`/`WEIGHTS_0`), and keyframe animation clips.
    ///
    /// # Errors
    /// [`Doc3dError`] when the glTF buffer is malformed or lacks triangle meshes.
    pub fn import_gltf(
        bytes: &[u8],
        base_dir: Option<&std::path::Path>,
    ) -> Result<Self, Doc3dError> {
        use base64::Engine;

        let gltf =
            gltf::Gltf::from_slice(bytes).map_err(|e| Doc3dError::Invalid(format!("glTF: {e}")))?;
        let data: Vec<Vec<u8>> = gltf
            .document
            .buffers()
            .map(|buffer| match buffer.source() {
                gltf::buffer::Source::Bin => gltf
                    .blob
                    .clone()
                    .ok_or_else(|| Doc3dError::Invalid("GLB buffer without a BIN chunk".into())),
                gltf::buffer::Source::Uri(uri) if uri.starts_with("data:") => {
                    let payload = uri
                        .split_once(";base64,")
                        .map(|(_, p)| p)
                        .ok_or_else(|| {
                            Doc3dError::Invalid("only base64 data URIs are supported".into())
                        })?;
                    base64::engine::general_purpose::STANDARD
                        .decode(payload)
                        .map_err(|e| Doc3dError::Invalid(format!("buffer base64: {e}")))
                }
                gltf::buffer::Source::Uri(uri) => {
                    let dir = base_dir.ok_or_else(|| {
                        Doc3dError::Invalid("external buffer without a directory".into())
                    })?;
                    if uri.contains("..")
                        || std::path::Path::new(uri).is_absolute()
                        || uri.contains(':')
                    {
                        return Err(Doc3dError::Invalid(format!(
                            "buffer uri outside the model directory: {uri}"
                        )));
                    }
                    std::fs::read(dir.join(uri))
                        .map_err(|e| Doc3dError::Invalid(format!("buffer I/O: {e}")))
                }
            })
            .collect::<Result<_, _>>()?;

        let mut doc = Self::default();
        let gdoc = &gltf.document;

        // Map glTF node index -> parent glTF node index.
        let mut node_parent: BTreeMap<usize, usize> = BTreeMap::new();
        for node in gdoc.nodes() {
            for child in node.children() {
                node_parent.insert(child.index(), node.index());
            }
        }

        // Reconstruct skeleton from the first skin if present.
        let mut joint_node_to_bone: BTreeMap<usize, usize> = BTreeMap::new();
        if let Some(skin) = gdoc.skins().next() {
            for (bi, joint) in skin.joints().enumerate() {
                joint_node_to_bone.insert(joint.index(), bi);
            }
            for (bi, joint) in skin.joints().enumerate() {
                let name = joint
                    .name()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("bone_{bi}"));
                let parent = node_parent
                    .get(&joint.index())
                    .and_then(|pi| joint_node_to_bone.get(pi).copied())
                    .filter(|&p| p < bi);
                let (t, r, s) = joint.transform().decomposed();
                doc.skeleton.bones.push(super::rig::Bone {
                    name,
                    parent,
                    rest: Trs {
                        translation: t,
                        rotation: Quat::from(r),
                        scale: s,
                    },
                    length: 0.25,
                });
            }
        }

        // Import mesh nodes.
        for node in gdoc.nodes() {
            let Some(mesh) = node.mesh() else { continue };
            let base_name = node
                .name()
                .or_else(|| mesh.name())
                .map(str::to_string)
                .unwrap_or_else(|| format!("object_{}", doc.objects.len()));
            for (pi, primitive) in mesh.primitives().enumerate() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    continue;
                }
                let reader = primitive.reader(|b| data.get(b.index()).map(Vec::as_slice));
                let Some(positions) = reader.read_positions() else {
                    continue;
                };
                let positions: Vec<[f32; 3]> = positions.collect();
                let indices: Vec<u32> = match reader.read_indices() {
                    Some(ind) => ind.into_u32().collect(),
                    None => (0..positions.len() as u32).collect(),
                };
                let faces: Vec<Vec<usize>> = indices
                    .chunks_exact(3)
                    .map(|t| vec![t[0] as usize, t[1] as usize, t[2] as usize])
                    .collect();
                let poly = PolyMesh::from_faces(positions.clone(), faces)?;

                let skin = match (reader.read_joints(0), reader.read_weights(0)) {
                    (Some(joints), Some(weights)) => {
                        let joints: Vec<[u16; 4]> = joints.into_u16().collect();
                        let weights: Vec<[f32; 4]> = weights.into_f32().collect();
                        (joints.len() == positions.len() && weights.len() == positions.len())
                            .then_some(SkinWeights { joints, weights })
                    }
                    _ => None,
                };

                let obj_name = if pi == 0 && doc.objects.iter().all(|o| o.name != base_name) {
                    base_name.clone()
                } else {
                    format!("{base_name}_{pi}")
                };
                doc.objects.push(Object3d {
                    name: obj_name,
                    mesh: poly,
                    transform: TransformSpec::default(),
                    material: Some("clay".into()),
                    shading: Shading::Smooth,
                    skin,
                    skinning: SkinningMethod::default(),
                    visible: true,
                });
            }
        }

        if doc.objects.is_empty() {
            return Err(Doc3dError::Invalid(
                "glTF scene has no triangle meshes".into(),
            ));
        }

        // Import animations.
        for (ai, anim) in gdoc.animations().enumerate() {
            let clip_name = anim
                .name()
                .map(str::to_string)
                .unwrap_or_else(|| format!("clip_{ai}"));
            let mut channels = Vec::new();
            for ch in anim.channels() {
                let target = ch.target();
                let Some(&bi) = joint_node_to_bone.get(&target.node().index()) else {
                    continue;
                };
                let bone_name = doc.skeleton.bones[bi].name.clone();
                let reader = ch.reader(|b| data.get(b.index()).map(Vec::as_slice));
                let Some(inputs) = reader.read_inputs() else {
                    continue;
                };
                let times: Vec<f32> = inputs.collect();
                let Some(outputs) = reader.read_outputs() else {
                    continue;
                };
                let (property, values): (Property, Vec<[f32; 4]>) = match outputs {
                    gltf::animation::util::ReadOutputs::Translations(it) => (
                        Property::Translation,
                        it.map(|v| [v[0], v[1], v[2], 0.0]).collect(),
                    ),
                    gltf::animation::util::ReadOutputs::Rotations(it) => {
                        (Property::Rotation, it.into_f32().collect())
                    }
                    gltf::animation::util::ReadOutputs::Scales(it) => (
                        Property::Scale,
                        it.map(|v| [v[0], v[1], v[2], 0.0]).collect(),
                    ),
                    gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => continue,
                };
                if times.is_empty() || times.len() != values.len() {
                    continue;
                }
                let interpolation = match ch.sampler().interpolation() {
                    gltf::animation::Interpolation::Step => Interpolation::Step,
                    gltf::animation::Interpolation::Linear => Interpolation::Linear,
                    gltf::animation::Interpolation::CubicSpline => Interpolation::Smooth,
                };
                channels.push(Channel {
                    bone: bone_name,
                    property,
                    interpolation,
                    times,
                    values,
                });
            }
            if !channels.is_empty() {
                doc.clips.push(Clip {
                    name: clip_name,
                    channels,
                    looping: true,
                });
            }
        }

        Ok(doc)
    }

    /// Summary for UIs and agents.
    #[must_use]
    pub fn stats(&self) -> DocumentStats {
        DocumentStats {
            objects: self
                .objects
                .iter()
                .map(|o| ObjectStats {
                    name: o.name.clone(),
                    topology: o.mesh.topology(),
                    volume: o.mesh.signed_volume(),
                    skinned: o.skin.is_some(),
                    material: o.material.clone(),
                })
                .collect(),
            bones: self.skeleton.bones.iter().map(|b| b.name.clone()).collect(),
            clips: self
                .clips
                .iter()
                .map(|c| (c.name.clone(), c.duration()))
                .collect(),
        }
    }
}

/// Atomic, undoable application of [`Op`]s.
#[derive(Clone, Debug, Default)]
pub struct Editor {
    /// Current document.
    pub doc: Document3d,
    undo: Vec<Document3d>,
    redo: Vec<Document3d>,
}

impl Editor {
    /// Wrap a document.
    #[must_use]
    pub fn new(doc: Document3d) -> Self {
        Self {
            doc,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Apply an op; on error the document is unchanged.
    ///
    /// # Errors
    /// [`Doc3dError`] from the op.
    pub fn apply(&mut self, op: &Op) -> Result<OpReport, Doc3dError> {
        let mut next = self.doc.clone();
        let report = next.apply(op)?;
        let prev = std::mem::replace(&mut self.doc, next);
        self.undo.push(prev);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        Ok(report)
    }

    /// Undo the last op; `false` when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(&mut self.doc, prev));
                true
            }
            None => false,
        }
    }

    /// Redo the last undone op; `false` when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(&mut self.doc, next));
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script() -> Vec<Op> {
        serde_json::from_str(include_str!("../../examples/studio3d/character.json"))
            .map(|s: serde_json::Value| serde_json::from_value(s["ops"].clone()).unwrap())
            .unwrap()
    }

    fn built() -> Editor {
        let mut e = Editor::default();
        for op in script() {
            e.apply(&op).unwrap_or_else(|err| panic!("{op:?}: {err}"));
        }
        e
    }

    #[test]
    fn character_script_builds_a_rigged_animated_model() {
        let e = built();
        let s = e.doc.stats();
        assert_eq!(s.bones.len(), 3);
        assert_eq!(s.clips.len(), 1);
        let body = s.objects.iter().find(|o| o.name == "body").unwrap();
        assert!(body.skinned);
        assert_eq!(body.topology.boundary_edges, 0);
        assert_eq!(body.topology.non_manifold_edges, 0);
        assert!(body.volume > 0.0);
    }

    #[test]
    fn animation_deforms_the_skinned_mesh() {
        let e = built();
        let rest = e.doc.nodes(None, 0.0).unwrap();
        let posed = e.doc.nodes(Some("wave"), 0.5).unwrap();
        let pos = |n: &Node3d| match n {
            Node3d::Mesh {
                geometry: Geometry::Mesh { positions, .. },
                ..
            } => positions.clone(),
            _ => panic!("expected mesh"),
        };
        let (a, b) = (pos(&rest[0]), pos(&posed[0]));
        assert_eq!(a.len(), b.len());
        let moved = a
            .iter()
            .zip(&b)
            .filter(|(p, q)| super::super::xform::length(super::super::xform::sub(**p, **q)) > 0.02)
            .count();
        assert!(moved > 0 && moved < a.len(), "{moved}/{}", a.len());
    }

    #[test]
    fn renders_and_exports() {
        let e = built();
        let view = ViewSpec {
            width: 96,
            height: 72,
            clip: Some("wave".into()),
            time: 0.25,
            ..ViewSpec::default()
        };
        let png = e.doc.render_png(&view).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        let canvas = crate::raster3d::render_scene3d(&e.doc.scene(&view).unwrap()).unwrap();
        // Something other than the background was drawn.
        let first = &canvas.pixels[0..4];
        assert!(canvas.pixels.chunks_exact(4).any(|p| p != first));
        let glb = e.doc.export_glb(&view).unwrap();
        assert_eq!(&glb[0..4], b"glTF");
        let back =
            crate::model::parse_gltf(&glb, None, MaterialDef::default().to_material().unwrap())
                .unwrap();
        assert!(!back.is_empty());
    }

    #[test]
    fn failed_ops_are_atomic_and_undo_redo_work() {
        let mut e = Editor::default();
        e.apply(&Op::AddObject {
            name: "a".into(),
            primitive: Primitive::Cube { size: [1.0; 3] },
            transform: TransformSpec::default(),
            material: Some("gold".into()),
            shading: Shading::Flat,
        })
        .unwrap();
        let before = e.doc.clone();
        assert!(
            e.apply(&Op::Extrude {
                object: "a".into(),
                faces: FaceSelection::Indices { faces: vec![0, 99] },
                distance: 1.0,
            })
            .is_err()
        );
        assert_eq!(e.doc, before);
        assert!(
            e.apply(&Op::AssignMaterial {
                object: "a".into(),
                material: "nope".into()
            })
            .is_err()
        );
        assert!(
            e.apply(&Op::Subdivide {
                object: "a".into(),
                levels: 9
            })
            .is_err()
        );
        e.apply(&Op::Subdivide {
            object: "a".into(),
            levels: 1,
        })
        .unwrap();
        assert_eq!(e.doc.objects[0].mesh.faces.len(), 24);
        assert!(e.undo());
        assert_eq!(e.doc, before);
        assert!(e.redo());
        assert_eq!(e.doc.objects[0].mesh.faces.len(), 24);
        assert!(!e.redo());
    }

    #[test]
    fn document_roundtrips_through_json() {
        let e = built();
        let json = serde_json::to_string(&e.doc).unwrap();
        let back: Document3d = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stats(), e.doc.stats());
    }

    #[test]
    fn skinned_glb_roundtrips_rig_weights_and_animations() {
        let e = built();
        let view = ViewSpec::default();
        let glb = e.doc.export_skinned_glb(&view).unwrap();
        assert_eq!(&glb[0..4], b"glTF");

        let imported = Document3d::import_gltf(&glb, None).unwrap();
        let stats = imported.stats();
        assert_eq!(stats.bones.len(), e.doc.stats().bones.len());
        assert_eq!(stats.clips.len(), e.doc.stats().clips.len());
        assert!(stats.objects.iter().any(|o| o.skinned));
    }

    #[test]
    fn bevel_loop_cut_bridge_geodesic_dqs_and_ik_ops_work() {
        use super::super::poly::{EdgeSelection, VertexSelection};

        let mut e = Editor::default();
        e.apply(&Op::AddObject {
            name: "box".into(),
            primitive: Primitive::Cube { size: [1.0; 3] },
            transform: TransformSpec::default(),
            material: Some("gold".into()),
            shading: Shading::Smooth,
        })
        .unwrap();
        e.apply(&Op::LoopCut {
            object: "box".into(),
            edge: [0, 1],
            factor: 0.5,
        })
        .unwrap();
        e.apply(&Op::Bevel {
            object: "box".into(),
            edges: EdgeSelection::All,
            vertices: VertexSelection::None,
            width: 0.15,
        })
        .unwrap();

        e.apply(&Op::AddBone {
            name: "root".into(),
            parent: None,
            rest: TransformSpec::default(),
            length: 0.5,
        })
        .unwrap();
        e.apply(&Op::AddBone {
            name: "tip".into(),
            parent: Some("root".into()),
            rest: TransformSpec {
                translation: [0.0, 0.5, 0.0],
                ..TransformSpec::default()
            },
            length: 0.5,
        })
        .unwrap();
        e.apply(&Op::BindSkinGeodesic {
            object: "box".into(),
            falloff: 2.0,
        })
        .unwrap();
        e.apply(&Op::SetSkinningMethod {
            object: "box".into(),
            method: SkinningMethod::DualQuaternion,
        })
        .unwrap();
        e.apply(&Op::SolveIk {
            chain: vec!["root".into(), "tip".into()],
            target: [0.4, 0.6, 0.0],
            tolerance: 1e-3,
            max_iterations: 16,
        })
        .unwrap();

        let nodes = e.doc.nodes(None, 0.0).unwrap();
        assert_eq!(nodes.len(), 1);
    }
}
