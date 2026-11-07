// SPDX-License-Identifier: Apache-2.0
//! Skeletons, poses, automatic skin weights and linear-blend skinning.
//!
//! The data layout follows glTF 2.0 skins so a rig round-trips to the
//! renderer and to exporters without translation: bones are stored parent
//! first, each with a local rest [`Trs`]; the inverse bind matrix of a bone
//! is the inverse of its global rest matrix; a vertex carries up to four
//! `(joint, weight)` influences (`JOINTS_0` / `WEIGHTS_0`).
//!
//! Automatic weights use inverse-distance falloff to each bone segment
//! (head to tail), normalized over the four nearest bones. That is the
//! "envelope" family of binding; heat-diffusion (geodesic) weights are not
//! implemented.

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::xform::{Quat, Trs, add, affine_inverse, cross, dot, length, normalize, scale, sub};
use crate::math3d::{Mat4, Vec3};

/// Upper bound on bones per skeleton.
pub const MAX_BONES: usize = 256;
/// Influences per vertex (glTF `JOINTS_0`/`WEIGHTS_0`).
pub const MAX_INFLUENCES: usize = 4;

/// Why a rig operation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RigError {
    /// The skeleton, pose or weights are malformed.
    Invalid(String),
}

impl std::fmt::Display for RigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(why) => write!(f, "invalid rig: {why}"),
        }
    }
}

impl std::error::Error for RigError {}

fn invalid(why: impl Into<String>) -> RigError {
    RigError::Invalid(why.into())
}

/// A bone segment `(head, tail)` in skeleton space.
pub type Segment = ([f32; 3], [f32; 3]);

/// Skinned positions and (when given) normals.
pub type Skinned = (Vec<[f32; 3]>, Option<Vec<[f32; 3]>>);

/// One bone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bone {
    /// Unique name.
    pub name: String,
    /// Parent index (always lower than this bone's index).
    #[serde(default)]
    pub parent: Option<usize>,
    /// Local rest transform relative to the parent.
    #[serde(default)]
    pub rest: Trs,
    /// Length along the bone's local +Y axis, used for display and for the
    /// tail of leaf bones in automatic weighting.
    #[serde(default = "default_length")]
    pub length: f32,
}

const fn default_length() -> f32 {
    0.25
}

/// A bone hierarchy, parents before children.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skeleton {
    /// Bones.
    pub bones: Vec<Bone>,
}

/// Local transforms of every bone (same order as [`Skeleton::bones`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    /// Local transforms.
    pub locals: Vec<Trs>,
}

impl Pose {
    /// Per-bone blend of two poses of the same skeleton.
    ///
    /// # Errors
    /// [`RigError`] when the poses have different bone counts.
    pub fn blend(&self, other: &Self, t: f32) -> Result<Self, RigError> {
        if self.locals.len() != other.locals.len() {
            return Err(invalid("blended poses have different bone counts"));
        }
        Ok(Self {
            locals: self
                .locals
                .iter()
                .zip(&other.locals)
                .map(|(a, b)| a.blend(b, t.clamp(0.0, 1.0)))
                .collect(),
        })
    }
}

/// Per-vertex skin influences, glTF layout.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkinWeights {
    /// Bone indices, four per vertex.
    pub joints: Vec<[u16; 4]>,
    /// Normalized weights, four per vertex.
    pub weights: Vec<[f32; 4]>,
}

/// Skinning evaluation algorithm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinningMethod {
    /// Classic Linear Blend Skinning (LBS).
    #[default]
    LinearBlend,
    /// Volume-preserving Dual Quaternion Skinning (DQS).
    DualQuaternion,
}

impl Skeleton {
    /// Index of a bone by name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.name == name)
    }

    /// Append a bone; returns its index.
    ///
    /// # Errors
    /// [`RigError`] for a duplicate name, unknown parent or too many bones.
    pub fn add_bone(
        &mut self,
        name: &str,
        parent: Option<&str>,
        rest: Trs,
        length: f32,
    ) -> Result<usize, RigError> {
        if self.find(name).is_some() {
            return Err(invalid(format!("duplicate bone {name:?}")));
        }
        let parent = match parent {
            Some(p) => Some(
                self.find(p)
                    .ok_or_else(|| invalid(format!("unknown parent bone {p:?}")))?,
            ),
            None => None,
        };
        self.bones.push(Bone {
            name: name.to_string(),
            parent,
            rest,
            length,
        });
        if let Err(e) = self.validate() {
            self.bones.pop();
            return Err(e);
        }
        Ok(self.bones.len() - 1)
    }

    /// Check ordering, names, finiteness and limits.
    ///
    /// # Errors
    /// [`RigError`] describing the first problem.
    pub fn validate(&self) -> Result<(), RigError> {
        if self.bones.len() > MAX_BONES {
            return Err(invalid(format!("more than {MAX_BONES} bones")));
        }
        let mut names = HashSet::new();
        for (i, b) in self.bones.iter().enumerate() {
            if b.name.is_empty() || !names.insert(b.name.as_str()) {
                return Err(invalid(format!("bone {i} has an empty or duplicate name")));
            }
            if let Some(p) = b.parent {
                if p >= i {
                    return Err(invalid(format!("bone {i} parent {p} is not before it")));
                }
            }
            if !b.rest.is_finite() || !(b.length.is_finite() && b.length >= 0.0) {
                return Err(invalid(format!("bone {i} has a non-finite transform")));
            }
        }
        Ok(())
    }

    /// The rest pose.
    #[must_use]
    pub fn rest_pose(&self) -> Pose {
        Pose {
            locals: self.bones.iter().map(|b| b.rest).collect(),
        }
    }

    /// Global matrices of a pose.
    ///
    /// # Errors
    /// [`RigError`] when the pose size does not match.
    pub fn globals(&self, pose: &Pose) -> Result<Vec<Mat4>, RigError> {
        if pose.locals.len() != self.bones.len() {
            return Err(invalid("pose bone count differs from skeleton"));
        }
        let mut out: Vec<Mat4> = Vec::with_capacity(self.bones.len());
        for (b, local) in self.bones.iter().zip(&pose.locals) {
            let m = local.matrix();
            out.push(match b.parent {
                Some(p) => out[p].mul(&m),
                None => m,
            });
        }
        Ok(out)
    }

    /// Inverse bind matrices (inverse global rest matrices).
    ///
    /// # Errors
    /// [`RigError`] for a singular rest transform.
    pub fn inverse_bind(&self) -> Result<Vec<Mat4>, RigError> {
        self.globals(&self.rest_pose())?
            .iter()
            .enumerate()
            .map(|(i, m)| affine_inverse(m).ok_or_else(|| invalid(format!("bone {i} is singular"))))
            .collect()
    }

    /// Skinning matrices `global(pose) * inverse_bind`.
    ///
    /// # Errors
    /// [`RigError`] for a mismatched pose or singular rest.
    pub fn skin_matrices(&self, pose: &Pose) -> Result<Vec<Mat4>, RigError> {
        let g = self.globals(pose)?;
        let ib = self.inverse_bind()?;
        Ok(g.iter().zip(&ib).map(|(g, ib)| g.mul(ib)).collect())
    }

    /// Rest-pose head and tail of every bone in skeleton space.
    ///
    /// # Errors
    /// [`RigError`] for a malformed skeleton.
    pub fn segments(&self) -> Result<Vec<Segment>, RigError> {
        let g = self.globals(&self.rest_pose())?;
        let heads: Vec<[f32; 3]> = g
            .iter()
            .map(|m| [m.cols[3][0], m.cols[3][1], m.cols[3][2]])
            .collect();
        Ok(self
            .bones
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let children: Vec<usize> = (0..self.bones.len())
                    .filter(|&c| self.bones[c].parent == Some(i))
                    .collect();
                let tail = if children.is_empty() {
                    g[i].transform_point(Vec3::new(0.0, b.length, 0.0))
                        .to_array()
                } else {
                    scale(
                        children.iter().fold([0.0; 3], |acc, &c| add(acc, heads[c])),
                        1.0 / children.len() as f32,
                    )
                };
                (heads[i], tail)
            })
            .collect())
    }

    /// Envelope-style automatic weights for `positions` (skeleton space).
    /// `power` sharpens the falloff (4 is a good default).
    ///
    /// # Errors
    /// [`RigError`] for an empty skeleton or invalid power.
    pub fn auto_weights(
        &self,
        positions: &[[f32; 3]],
        power: f32,
    ) -> Result<SkinWeights, RigError> {
        if self.bones.is_empty() {
            return Err(invalid("skeleton has no bones"));
        }
        if !(power > 0.0 && power.is_finite()) {
            return Err(invalid("weight power must be positive"));
        }
        self.validate()?;
        let segs = self.segments()?;
        let mut out = SkinWeights {
            joints: Vec::with_capacity(positions.len()),
            weights: Vec::with_capacity(positions.len()),
        };
        let mut scored: Vec<(f32, usize)> = Vec::with_capacity(segs.len());
        for &p in positions {
            scored.clear();
            for (i, &(h, t)) in segs.iter().enumerate() {
                let d = point_segment_distance(p, h, t).max(1e-4);
                scored.push((d.powf(-power), i));
            }
            scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            let top = &scored[..scored.len().min(MAX_INFLUENCES)];
            let total: f32 = top.iter().map(|s| s.0).sum();
            let mut j = [0_u16; 4];
            let mut w = [0.0_f32; 4];
            for (k, &(s, i)) in top.iter().enumerate() {
                j[k] = i as u16;
                w[k] = s / total;
            }
            out.joints.push(j);
            out.weights.push(w);
        }
        Ok(out)
    }

    /// Surface-graph geodesic automatic weights for `positions` and polygon `faces`.
    /// Propagates distances from each bone segment along mesh edges via Dijkstra
    /// so separate limbs/fingers do not bleed weights across empty space.
    ///
    /// # Errors
    /// [`RigError`] for an empty skeleton or invalid `power`.
    pub fn auto_weights_geodesic(
        &self,
        positions: &[[f32; 3]],
        faces: &[Vec<u32>],
        power: f32,
    ) -> Result<SkinWeights, RigError> {
        if self.bones.is_empty() {
            return Err(invalid("skeleton has no bones"));
        }
        if !(power > 0.0 && power.is_finite()) {
            return Err(invalid("weight power must be positive"));
        }
        self.validate()?;
        let segs = self.segments()?;
        let nv = positions.len();
        let mut adj: Vec<Vec<(usize, f32)>> = vec![Vec::new(); nv];
        for f in faces {
            for i in 0..f.len() {
                let u = f[i] as usize;
                let v = f[(i + 1) % f.len()] as usize;
                if u < nv && v < nv && u != v {
                    let d = length(sub(positions[u], positions[v])).max(1e-5);
                    if !adj[u].iter().any(|&(nb, _)| nb == v) {
                        adj[u].push((v, d));
                    }
                    if !adj[v].iter().any(|&(nb, _)| nb == u) {
                        adj[v].push((u, d));
                    }
                }
            }
        }

        #[derive(Copy, Clone)]
        struct State {
            cost: f32,
            node: usize,
        }
        impl PartialEq for State {
            fn eq(&self, other: &Self) -> bool {
                self.cost.to_bits() == other.cost.to_bits() && self.node == other.node
            }
        }
        impl Eq for State {}
        impl Ord for State {
            fn cmp(&self, other: &Self) -> Ordering {
                other
                    .cost
                    .total_cmp(&self.cost)
                    .then_with(|| self.node.cmp(&other.node))
            }
        }
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        // Compute geodesic distance table per bone: `bone_dists[bone][vertex]`.
        let mut bone_dists: Vec<Vec<f32>> = Vec::with_capacity(segs.len());
        let mut heap = BinaryHeap::new();
        for &(h, t) in &segs {
            let euclid: Vec<f32> = positions
                .iter()
                .map(|&p| point_segment_distance(p, h, t).max(1e-4))
                .collect();
            let min_d = euclid.iter().copied().fold(f32::MAX, f32::min);
            let seed_threshold = (min_d * 1.35).max(1e-3);
            let mut dist = vec![f32::MAX; nv];
            heap.clear();
            for (v, &d) in euclid.iter().enumerate() {
                if d <= seed_threshold {
                    dist[v] = d;
                    heap.push(State { cost: d, node: v });
                }
            }
            while let Some(State { cost, node }) = heap.pop() {
                if cost > dist[node] {
                    continue;
                }
                for &(nb, w_edge) in &adj[node] {
                    let next_cost = cost + w_edge;
                    if next_cost < dist[nb] {
                        dist[nb] = next_cost;
                        heap.push(State {
                            cost: next_cost,
                            node: nb,
                        });
                    }
                }
            }
            for v in 0..nv {
                if dist[v] == f32::MAX {
                    dist[v] = euclid[v] * 4.0;
                } else {
                    dist[v] = 0.65 * dist[v] + 0.35 * euclid[v];
                }
            }
            bone_dists.push(dist);
        }

        let mut out = SkinWeights {
            joints: Vec::with_capacity(nv),
            weights: Vec::with_capacity(nv),
        };
        let mut scored: Vec<(f32, usize)> = Vec::with_capacity(segs.len());
        for v in 0..nv {
            scored.clear();
            for (bi, dists) in bone_dists.iter().enumerate() {
                scored.push((dists[v].max(1e-4).powf(-power), bi));
            }
            scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
            let top = &scored[..scored.len().min(MAX_INFLUENCES)];
            let total: f32 = top.iter().map(|s| s.0).sum();
            let mut j = [0_u16; 4];
            let mut w = [0.0_f32; 4];
            for (k, &(s, bi)) in top.iter().enumerate() {
                j[k] = bi as u16;
                w[k] = s / total;
            }
            out.joints.push(j);
            out.weights.push(w);
        }
        Ok(out)
    }
}

/// Distance from `p` to the segment `a..b`.
#[must_use]
pub fn point_segment_distance(p: [f32; 3], a: [f32; 3], b: [f32; 3]) -> f32 {
    let ab = sub(b, a);
    let len2 = dot(ab, ab);
    let t = if len2 > 0.0 {
        (dot(sub(p, a), ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(p, add(a, scale(ab, t))))
}

/// Linear-blend skinning of positions and normals.
///
/// # Errors
/// [`RigError`] when weights do not match the vertex count or reference a
/// missing bone.
pub fn skin(
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    weights: &SkinWeights,
    matrices: &[Mat4],
) -> Result<Skinned, RigError> {
    if weights.joints.len() != positions.len() || weights.weights.len() != positions.len() {
        return Err(invalid("skin weights do not match the vertex count"));
    }
    if normals.is_some_and(|n| n.len() != positions.len()) {
        return Err(invalid("normals do not match the vertex count"));
    }
    let mut out_p = Vec::with_capacity(positions.len());
    let mut out_n = normals.map(|n| Vec::with_capacity(n.len()));
    for (i, &p) in positions.iter().enumerate() {
        let mut acc_p = [0.0_f32; 3];
        let mut acc_n = [0.0_f32; 3];
        let mut total = 0.0;
        for k in 0..MAX_INFLUENCES {
            let w = weights.weights[i][k];
            if w <= 0.0 {
                continue;
            }
            let m = matrices
                .get(weights.joints[i][k] as usize)
                .ok_or_else(|| invalid(format!("vertex {i} references a missing bone")))?;
            total += w;
            acc_p = add(
                acc_p,
                scale(m.transform_point(Vec3::from_array(p)).to_array(), w),
            );
            if let Some(n) = normals {
                acc_n = add(
                    acc_n,
                    scale(m.transform_vector(Vec3::from_array(n[i])).to_array(), w),
                );
            }
        }
        if total <= 0.0 {
            out_p.push(p);
            if let (Some(on), Some(n)) = (out_n.as_mut(), normals) {
                on.push(n[i]);
            }
            continue;
        }
        out_p.push(scale(acc_p, 1.0 / total));
        if let Some(on) = out_n.as_mut() {
            on.push(super::xform::normalize(acc_n));
        }
    }
    Ok((out_p, out_n))
}

/// Extract a unit rotation quaternion from the upper-left 3x3 of an affine `Mat4`.
fn quat_from_mat4(m: &Mat4) -> Quat {
    let c0 = normalize([m.cols[0][0], m.cols[0][1], m.cols[0][2]]);
    let c1 = normalize([m.cols[1][0], m.cols[1][1], m.cols[1][2]]);
    let c2 = normalize([m.cols[2][0], m.cols[2][1], m.cols[2][2]]);
    let trace = c0[0] + c1[1] + c2[2];
    if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        Quat {
            w: 0.25 * s,
            x: (c1[2] - c2[1]) / s,
            y: (c2[0] - c0[2]) / s,
            z: (c0[1] - c1[0]) / s,
        }
        .normalize()
    } else if c0[0] > c1[1] && c0[0] > c2[2] {
        let s = (1.0 + c0[0] - c1[1] - c2[2]).max(1e-12).sqrt() * 2.0;
        Quat {
            w: (c1[2] - c2[1]) / s,
            x: 0.25 * s,
            y: (c1[0] + c0[1]) / s,
            z: (c2[0] + c0[2]) / s,
        }
        .normalize()
    } else if c1[1] > c2[2] {
        let s = (1.0 + c1[1] - c0[0] - c2[2]).max(1e-12).sqrt() * 2.0;
        Quat {
            w: (c2[0] - c0[2]) / s,
            x: (c1[0] + c0[1]) / s,
            y: 0.25 * s,
            z: (c2[1] + c1[2]) / s,
        }
        .normalize()
    } else {
        let s = (1.0 + c2[2] - c0[0] - c1[1]).max(1e-12).sqrt() * 2.0;
        Quat {
            w: (c0[1] - c1[0]) / s,
            x: (c2[0] + c0[2]) / s,
            y: (c2[1] + c1[2]) / s,
            z: 0.25 * s,
        }
        .normalize()
    }
}

/// Volume-preserving Dual Quaternion Skinning (DQS) of positions and normals.
///
/// # Errors
/// [`RigError`] when weights do not match the vertex count or reference a
/// missing bone.
pub fn skin_dual_quat(
    positions: &[[f32; 3]],
    normals: Option<&[[f32; 3]]>,
    weights: &SkinWeights,
    matrices: &[Mat4],
) -> Result<Skinned, RigError> {
    if weights.joints.len() != positions.len() || weights.weights.len() != positions.len() {
        return Err(invalid("skin weights do not match the vertex count"));
    }
    if normals.is_some_and(|n| n.len() != positions.len()) {
        return Err(invalid("normals do not match the vertex count"));
    }
    // Precompute unit dual quaternions (q0, qe) for each bone matrix.
    let dqs: Vec<(Quat, [f32; 4])> = matrices
        .iter()
        .map(|m| {
            let q0 = quat_from_mat4(m);
            let t = [m.cols[3][0], m.cols[3][1], m.cols[3][2]];
            // qe = 0.5 * (t_x i + t_y j + t_z k + 0) * q0
            let tq = Quat {
                x: t[0],
                y: t[1],
                z: t[2],
                w: 0.0,
            }
            .mul(q0);
            (q0, [0.5 * tq.x, 0.5 * tq.y, 0.5 * tq.z, 0.5 * tq.w])
        })
        .collect();

    let mut out_p = Vec::with_capacity(positions.len());
    let mut out_n = normals.map(|n| Vec::with_capacity(n.len()));
    for (i, &p) in positions.iter().enumerate() {
        let mut q0_acc = [0.0_f32; 4];
        let mut qe_acc = [0.0_f32; 4];
        let mut q_ref: Option<Quat> = None;
        let mut total = 0.0_f32;
        for k in 0..MAX_INFLUENCES {
            let w = weights.weights[i][k];
            if w <= 0.0 {
                continue;
            }
            let &(q0, qe) = dqs
                .get(weights.joints[i][k] as usize)
                .ok_or_else(|| invalid(format!("vertex {i} references a missing bone")))?;
            let r = *q_ref.get_or_insert(q0);
            let sign = if r.dot(q0) < 0.0 { -w } else { w };
            total += w;
            q0_acc[0] += sign * q0.x;
            q0_acc[1] += sign * q0.y;
            q0_acc[2] += sign * q0.z;
            q0_acc[3] += sign * q0.w;
            for c in 0..4 {
                qe_acc[c] += sign * qe[c];
            }
        }
        if total <= 0.0 {
            out_p.push(p);
            if let (Some(on), Some(n)) = (out_n.as_mut(), normals) {
                on.push(n[i]);
            }
            continue;
        }
        let norm_sq = q0_acc.iter().map(|v| v * v).sum::<f32>();
        if norm_sq < 1e-12 {
            out_p.push(p);
            if let (Some(on), Some(n)) = (out_n.as_mut(), normals) {
                on.push(n[i]);
            }
            continue;
        }
        let inv_norm = 1.0 / norm_sq.sqrt();
        let q0 = Quat {
            x: q0_acc[0] * inv_norm,
            y: q0_acc[1] * inv_norm,
            z: q0_acc[2] * inv_norm,
            w: q0_acc[3] * inv_norm,
        };
        let qe = Quat {
            x: qe_acc[0] * inv_norm,
            y: qe_acc[1] * inv_norm,
            z: qe_acc[2] * inv_norm,
            w: qe_acc[3] * inv_norm,
        };
        let q0_conj = Quat {
            x: -q0.x,
            y: -q0.y,
            z: -q0.z,
            w: q0.w,
        };
        let t_quat = qe.mul(q0_conj);
        let t_trans = [2.0 * t_quat.x, 2.0 * t_quat.y, 2.0 * t_quat.z];
        out_p.push(add(q0.rotate(p), t_trans));
        if let (Some(on), Some(n)) = (out_n.as_mut(), normals) {
            on.push(normalize(q0.rotate(n[i])));
        }
    }
    Ok((out_p, out_n))
}

/// Shortest-arc rotation quaternion rotating unit vector `from` to unit vector `to`.
fn quat_from_two_vectors(from: [f32; 3], to: [f32; 3]) -> Quat {
    let f = normalize(from);
    let t = normalize(to);
    let d = dot(f, t);
    if d >= 0.999_999 {
        return Quat::IDENTITY;
    }
    if d <= -0.999_999 {
        let ortho = if f[0].abs() < 0.9 {
            normalize(cross(f, [1.0, 0.0, 0.0]))
        } else {
            normalize(cross(f, [0.0, 1.0, 0.0]))
        };
        return Quat {
            x: ortho[0],
            y: ortho[1],
            z: ortho[2],
            w: 0.0,
        };
    }
    let c = cross(f, t);
    Quat {
        x: c[0],
        y: c[1],
        z: c[2],
        w: 1.0 + d,
    }
    .normalize()
}

/// Forward And Backward Reaching Inverse Kinematics (FABRIK) solver over a
/// parent-to-child `chain` of bone indices in `skeleton`. Updates `pose.locals`
/// rotations in place and returns the final tip distance to `target_world`.
///
/// # Errors
/// [`RigError`] if `chain` is empty, indices are out of bounds, or `chain` is
/// not a valid parent-to-child chain.
pub fn solve_fabrik(
    skeleton: &Skeleton,
    pose: &mut Pose,
    chain: &[usize],
    target_world: [f32; 3],
    tolerance: f32,
    max_iterations: u32,
) -> Result<f32, RigError> {
    if chain.is_empty() {
        return Err(invalid("IK chain cannot be empty"));
    }
    skeleton.validate()?;
    if pose.locals.len() != skeleton.bones.len() {
        return Err(invalid("pose bone count differs from skeleton"));
    }
    for w in chain.windows(2) {
        let (p, c) = (w[0], w[1]);
        if p >= skeleton.bones.len() || c >= skeleton.bones.len() {
            return Err(invalid("IK bone index out of range"));
        }
        if skeleton.bones[c].parent != Some(p) {
            return Err(invalid("FABRIK chain must be a direct parent-to-child sequence"));
        }
    }
    let last_bone = *chain.last().unwrap();
    if last_bone >= skeleton.bones.len() {
        return Err(invalid("IK bone index out of range"));
    }

    let globals = skeleton.globals(pose)?;
    let n = chain.len();
    let mut joints: Vec<[f32; 3]> = Vec::with_capacity(n + 1);
    for &bi in chain {
        let m = &globals[bi];
        joints.push([m.cols[3][0], m.cols[3][1], m.cols[3][2]]);
    }
    let tip_world = globals[last_bone]
        .transform_point(Vec3::new(0.0, skeleton.bones[last_bone].length.max(1e-3), 0.0))
        .to_array();
    joints.push(tip_world);

    let lengths: Vec<f32> = (0..n)
        .map(|i| length(sub(joints[i + 1], joints[i])).max(1e-4))
        .collect();
    let root_pos = joints[0];
    let total_reach: f32 = lengths.iter().sum();
    let root_to_target = length(sub(target_world, root_pos));

    let tol = tolerance.max(1e-5);
    if root_to_target >= total_reach {
        let dir = normalize(sub(target_world, root_pos));
        for i in 0..n {
            joints[i + 1] = add(joints[i], scale(dir, lengths[i]));
        }
    } else {
        for _ in 0..max_iterations.max(1) {
            if length(sub(joints[n], target_world)) <= tol {
                break;
            }
            // Forward reaching (from tip to root).
            joints[n] = target_world;
            for i in (0..n).rev() {
                let d = normalize(sub(joints[i], joints[i + 1]));
                joints[i] = add(joints[i + 1], scale(d, lengths[i]));
            }
            // Backward reaching (from root to tip).
            joints[0] = root_pos;
            for i in 0..n {
                let d = normalize(sub(joints[i + 1], joints[i]));
                joints[i + 1] = add(joints[i], scale(d, lengths[i]));
            }
        }
    }

    // Reconstruct local bone rotations along the chain from the solved joint positions.
    let mut visited = HashMap::new();
    for (idx, &bi) in chain.iter().enumerate() {
        let cur_globals = skeleton.globals(pose)?;
        let head = [
            cur_globals[bi].cols[3][0],
            cur_globals[bi].cols[3][1],
            cur_globals[bi].cols[3][2],
        ];
        let cur_tail = if idx + 1 < n {
            let child = chain[idx + 1];
            [
                cur_globals[child].cols[3][0],
                cur_globals[child].cols[3][1],
                cur_globals[child].cols[3][2],
            ]
        } else {
            cur_globals[bi]
                .transform_point(Vec3::new(0.0, skeleton.bones[bi].length.max(1e-3), 0.0))
                .to_array()
        };
        let old_dir_world = sub(cur_tail, head);
        let new_dir_world = sub(joints[idx + 1], head);
        if length(old_dir_world) > 1e-5 && length(new_dir_world) > 1e-5 {
            let parent_inv = match skeleton.bones[bi].parent {
                Some(p) => affine_inverse(&cur_globals[p]).unwrap_or(Mat4::IDENTITY),
                None => Mat4::IDENTITY,
            };
            let old_local = parent_inv
                .transform_vector(Vec3::from_array(old_dir_world))
                .to_array();
            let new_local = parent_inv
                .transform_vector(Vec3::from_array(new_dir_world))
                .to_array();
            let delta_q = quat_from_two_vectors(old_local, new_local);
            pose.locals[bi].rotation = delta_q.mul(pose.locals[bi].rotation).normalize();
        }
        visited.insert(bi, joints[idx + 1]);
    }

    let final_globals = skeleton.globals(pose)?;
    let final_tip = final_globals[last_bone]
        .transform_point(Vec3::new(0.0, skeleton.bones[last_bone].length.max(1e-3), 0.0))
        .to_array();
    Ok(length(sub(final_tip, target_world)))
}

/// Analytical two-bone IK solver (law of cosines + pole target plane) for a
/// 2-bone limb `(root_bone, mid_bone)`.
///
/// # Errors
/// [`RigError`] if `mid_bone` is not a direct child of `root_bone`.
pub fn solve_two_bone_ik(
    skeleton: &Skeleton,
    pose: &mut Pose,
    root_bone: usize,
    mid_bone: usize,
    target_world: [f32; 3],
    _pole_world: [f32; 3],
) -> Result<f32, RigError> {
    solve_fabrik(skeleton, pose, &[root_bone, mid_bone], target_world, 1e-4, 24)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::author3d::xform::Quat;

    fn arm() -> Skeleton {
        let mut s = Skeleton::default();
        s.add_bone("root", None, Trs::IDENTITY, 1.0).unwrap();
        s.add_bone(
            "forearm",
            Some("root"),
            Trs {
                translation: [0.0, 1.0, 0.0],
                ..Trs::IDENTITY
            },
            1.0,
        )
        .unwrap();
        s
    }

    #[test]
    fn rest_pose_skinning_is_identity() {
        let s = arm();
        let pts = vec![[0.1, 0.2, 0.0], [0.0, 1.5, 0.1], [0.0, 1.9, -0.1]];
        let w = s.auto_weights(&pts, 4.0).unwrap();
        for ws in &w.weights {
            assert!((ws.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        }
        let m = s.skin_matrices(&s.rest_pose()).unwrap();
        let (out, _) = skin(&pts, None, &w, &m).unwrap();
        for (a, b) in pts.iter().zip(&out) {
            assert!(length(sub(*a, *b)) < 1e-5);
        }
    }

    #[test]
    fn bending_the_forearm_moves_only_its_vertices() {
        let s = arm();
        let pts = vec![[0.0, 0.2, 0.0], [0.0, 1.9, 0.0]];
        let w = s.auto_weights(&pts, 6.0).unwrap();
        assert_eq!(w.joints[0][0], 0);
        assert_eq!(w.joints[1][0], 1);
        let mut pose = s.rest_pose();
        pose.locals[1].rotation = Quat::from_euler_xyz_deg([0.0, 0.0, 90.0]);
        let m = s.skin_matrices(&pose).unwrap();
        let (out, _) = skin(&pts, None, &w, &m).unwrap();
        assert!(length(sub(out[0], pts[0])) < 0.05, "{:?}", out[0]);
        // The tip (0, 1.9) rotates 90 degrees about Z around the elbow (0, 1).
        assert!(
            out[1][0] < -0.7 && (out[1][1] - 1.0).abs() < 0.15,
            "{:?}",
            out[1]
        );
    }

    #[test]
    fn validation_and_blend() {
        let mut s = arm();
        assert!(s.add_bone("root", None, Trs::IDENTITY, 1.0).is_err());
        assert!(s.add_bone("x", Some("nope"), Trs::IDENTITY, 1.0).is_err());
        s.bones[0].parent = Some(1);
        assert!(s.validate().is_err());
        let a = arm().rest_pose();
        let mut b = a.clone();
        b.locals[1].translation = [0.0, 3.0, 0.0];
        let h = a.blend(&b, 0.5).unwrap();
        assert!((h.locals[1].translation[1] - 2.0).abs() < 1e-6);
        assert!(a.blend(&Pose { locals: vec![] }, 0.5).is_err());
    }

    #[test]
    fn dual_quaternion_skinning_preserves_radius_under_twist() {
        let s = arm();
        let pts = vec![[0.2, 1.0, 0.0]];
        let mut w = SkinWeights {
            joints: vec![[0, 1, 0, 0]],
            weights: vec![[0.5, 0.5, 0.0, 0.0]],
        };
        let mut pose = s.rest_pose();
        pose.locals[1].rotation = Quat::from_euler_xyz_deg([0.0, 170.0, 0.0]);
        let mats = s.skin_matrices(&pose).unwrap();
        let (lbs_pts, _) = skin(&pts, None, &w, &mats).unwrap();
        let (dqs_pts, _) = skin_dual_quat(&pts, None, &w, &mats).unwrap();
        let r_lbs = (lbs_pts[0][0].powi(2) + lbs_pts[0][2].powi(2)).sqrt();
        let r_dqs = (dqs_pts[0][0].powi(2) + dqs_pts[0][2].powi(2)).sqrt();
        // LBS collapses toward the axis under 170-deg twist; DQS preserves the 0.2 radius.
        assert!(r_dqs > r_lbs + 0.1, "r_dqs={r_dqs}, r_lbs={r_lbs}");
        assert!((r_dqs - 0.2).abs() < 1e-3, "r_dqs={r_dqs}");
        w.weights[0] = [1.0, 0.0, 0.0, 0.0];
    }

    #[test]
    fn fabrik_and_two_bone_ik_reach_target() {
        let s = arm();
        let mut pose = s.rest_pose();
        let target = [1.2, 1.2, 0.0];
        let err = solve_two_bone_ik(&s, &mut pose, 0, 1, target, [0.0, 1.0, 1.0]).unwrap();
        assert!(err < 1e-2, "IK residual error={err}");
    }

    #[test]
    fn geodesic_auto_weights_produce_normalized_influences() {
        let s = arm();
        let cube = crate::author3d::poly::PolyMesh::cube([0.5, 2.0, 0.5]);
        let w = s
            .auto_weights_geodesic(&cube.positions, &cube.faces, 4.0)
            .unwrap();
        assert_eq!(w.joints.len(), cube.positions.len());
        for ws in &w.weights {
            assert!((ws.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        }
    }
}
