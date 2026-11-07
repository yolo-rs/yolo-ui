// SPDX-License-Identifier: Apache-2.0
//! Editable polygon meshes and the modeling operators that work on them.
//!
//! A [`PolyMesh`] keeps n-gon faces (counter-clockwise seen from outside) over
//! a shared vertex array, so modeling operators see the real topology instead
//! of a triangle soup: region extrusion, per-face inset, Catmull-Clark
//! subdivision, merge-by-distance, ray picking and topology reports. Output
//! goes back to the renderer as [`Geometry::Mesh`] through
//! [`PolyMesh::to_geometry`]. Faces are triangulated as fans, which is exact
//! for the convex faces every operator here produces; concave imported
//! n-gons should be triangulated before import.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::xform::{add, cross, dot, length, normalize, scale, sub};
use crate::{
    math3d::{Mat4, Vec3},
    mesh3d::Geometry,
};

/// Upper bound on the vertices of one editable mesh.
pub const MAX_POLY_VERTICES: usize = 2 * 1024 * 1024;
/// Upper bound on the faces of one editable mesh.
pub const MAX_POLY_FACES: usize = 2 * 1024 * 1024;

/// Why a modeling operation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolyError {
    /// The mesh or an argument is malformed.
    Invalid(String),
    /// The result would exceed [`MAX_POLY_VERTICES`] / [`MAX_POLY_FACES`].
    TooLarge(&'static str),
}

impl std::fmt::Display for PolyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(why) => write!(f, "invalid mesh operation: {why}"),
            Self::TooLarge(what) => write!(f, "mesh operation result too large: {what}"),
        }
    }
}

impl std::error::Error for PolyError {}

fn invalid(why: impl Into<String>) -> PolyError {
    PolyError::Invalid(why.into())
}

/// How [`PolyMesh::to_geometry`] shades faces.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shading {
    /// Shared vertices with area-weighted normals.
    #[default]
    Smooth,
    /// One normal per face (hard edges everywhere).
    Flat,
}

/// Edge/face counts and manifold diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Topology {
    /// Vertices referenced by at least one face.
    pub vertices: usize,
    /// Undirected edges.
    pub edges: usize,
    /// Faces.
    pub faces: usize,
    /// Edges used by exactly one face (open borders).
    pub boundary_edges: usize,
    /// Edges used by more than two faces.
    pub non_manifold_edges: usize,
    /// Euler characteristic `V - E + F` (2 for a closed genus-0 surface).
    pub euler: i64,
}

/// A ray/mesh intersection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    /// Face hit.
    pub face: usize,
    /// Distance along the (normalized) ray direction.
    pub distance: f32,
    /// Hit point.
    pub point: [f32; 3],
}

/// An editable polygon mesh.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PolyMesh {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Faces as counter-clockwise vertex loops (at least three vertices).
    pub faces: Vec<Vec<u32>>,
}

/// Explicit vertex index selection for modeling operators.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VertexSelection {
    /// Selected vertex indices.
    pub vertices: Vec<u32>,
}

/// Explicit undirected edge selection (`[u, v]`) for modeling operators.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeSelection {
    /// Selected undirected edges `[u, v]`.
    pub edges: Vec<[u32; 2]>,
}

type EdgeKey = (u32, u32);

fn key(a: u32, b: u32) -> EdgeKey {
    if a < b { (a, b) } else { (b, a) }
}

impl PolyMesh {
    /// Axis-aligned box centred on the origin, six quads.
    #[must_use]
    pub fn cube(size: [f32; 3]) -> Self {
        let [x, y, z] = [size[0] * 0.5, size[1] * 0.5, size[2] * 0.5];
        Self {
            positions: vec![
                [-x, -y, -z],
                [x, -y, -z],
                [x, y, -z],
                [-x, y, -z],
                [-x, -y, z],
                [x, -y, z],
                [x, y, z],
                [-x, y, z],
            ],
            faces: vec![
                vec![4, 5, 6, 7],
                vec![1, 0, 3, 2],
                vec![5, 1, 2, 6],
                vec![0, 4, 7, 3],
                vec![7, 6, 2, 3],
                vec![0, 1, 5, 4],
            ],
        }
    }

    /// Quad grid in the XZ plane facing +Y.
    ///
    /// # Errors
    /// [`PolyError`] for non-positive sizes or too many divisions.
    pub fn grid(size: [f32; 2], divisions: [u32; 2]) -> Result<Self, PolyError> {
        let [nx, nz] = [divisions[0].max(1) as usize, divisions[1].max(1) as usize];
        if !(size[0] > 0.0 && size[1] > 0.0 && size[0].is_finite() && size[1].is_finite()) {
            return Err(invalid("grid size must be positive"));
        }
        if (nx + 1) * (nz + 1) > MAX_POLY_VERTICES {
            return Err(PolyError::TooLarge("grid vertices"));
        }
        let mut positions = Vec::with_capacity((nx + 1) * (nz + 1));
        for j in 0..=nz {
            for i in 0..=nx {
                positions.push([
                    -size[0] * 0.5 + size[0] * i as f32 / nx as f32,
                    0.0,
                    -size[1] * 0.5 + size[1] * j as f32 / nz as f32,
                ]);
            }
        }
        let at = |i: usize, j: usize| (j * (nx + 1) + i) as u32;
        let mut faces = Vec::with_capacity(nx * nz);
        for j in 0..nz {
            for i in 0..nx {
                faces.push(vec![at(i, j), at(i, j + 1), at(i + 1, j + 1), at(i + 1, j)]);
            }
        }
        Ok(Self { positions, faces })
    }

    /// All-quad sphere: a cube subdivided `level` times and projected onto
    /// the sphere (even vertex density, the usual sculpting base).
    ///
    /// # Errors
    /// [`PolyError`] when the level is above 7 or the radius invalid.
    pub fn quad_sphere(radius: f32, level: u32) -> Result<Self, PolyError> {
        if !(radius > 0.0 && radius.is_finite()) {
            return Err(invalid("sphere radius must be positive"));
        }
        if level > 7 {
            return Err(PolyError::TooLarge("quad sphere level above 7"));
        }
        let mut mesh = Self::cube([2.0, 2.0, 2.0]);
        for _ in 0..level {
            mesh = mesh.catmull_clark()?;
        }
        for p in &mut mesh.positions {
            *p = scale(normalize(*p), radius);
        }
        Ok(mesh)
    }

    /// Import any renderer geometry (procedural or explicit), welding
    /// coincident vertices so the result has connected topology.
    ///
    /// # Errors
    /// [`PolyError`] when the geometry does not tessellate.
    pub fn from_geometry(geometry: &Geometry) -> Result<Self, PolyError> {
        let data = geometry.tessellate().map_err(|e| invalid(e.to_string()))?;
        Self::from_triangles(&data.positions, &data.indices, 1e-5)
    }

    /// Build from an indexed triangle list, welding vertices closer than
    /// `weld` and dropping degenerate triangles.
    ///
    /// # Errors
    /// [`PolyError`] for out-of-range indices or oversized input.
    pub fn from_triangles(
        positions: &[[f32; 3]],
        indices: &[u32],
        weld: f32,
    ) -> Result<Self, PolyError> {
        if positions.len() > MAX_POLY_VERTICES || indices.len() / 3 > MAX_POLY_FACES {
            return Err(PolyError::TooLarge("triangle input"));
        }
        if indices.len() % 3 != 0 {
            return Err(invalid("index count is not a multiple of three"));
        }
        let (positions, remap) = weld_positions(positions, weld);
        let mut faces = Vec::with_capacity(indices.len() / 3);
        for tri in indices.chunks_exact(3) {
            let mut f = Vec::with_capacity(3);
            for &i in tri {
                let mapped = *remap
                    .get(i as usize)
                    .ok_or_else(|| invalid(format!("index {i} out of range")))?;
                f.push(mapped);
            }
            if f[0] != f[1] && f[1] != f[2] && f[0] != f[2] {
                faces.push(f);
            }
        }
        let mut mesh = Self { positions, faces };
        mesh.compact();
        Ok(mesh)
    }

    /// Check indices, face sizes, finiteness and limits.
    ///
    /// # Errors
    /// [`PolyError`] describing the first problem.
    pub fn validate(&self) -> Result<(), PolyError> {
        if self.positions.len() > MAX_POLY_VERTICES {
            return Err(PolyError::TooLarge("vertices"));
        }
        if self.faces.len() > MAX_POLY_FACES {
            return Err(PolyError::TooLarge("faces"));
        }
        if let Some(i) = self
            .positions
            .iter()
            .position(|p| !p.iter().all(|c| c.is_finite()))
        {
            return Err(invalid(format!("vertex {i} is not finite")));
        }
        let n = self.positions.len() as u32;
        for (fi, f) in self.faces.iter().enumerate() {
            if f.len() < 3 {
                return Err(invalid(format!("face {fi} has fewer than three vertices")));
            }
            if let Some(&bad) = f.iter().find(|&&v| v >= n) {
                return Err(invalid(format!("face {fi} references vertex {bad}")));
            }
            let unique: HashSet<u32> = f.iter().copied().collect();
            if unique.len() != f.len() {
                return Err(invalid(format!("face {fi} repeats a vertex")));
            }
        }
        Ok(())
    }

    fn newell(&self, f: &[u32]) -> [f32; 3] {
        let mut n = [0.0_f32; 3];
        for i in 0..f.len() {
            let a = self.positions[f[i] as usize];
            let b = self.positions[f[(i + 1) % f.len()] as usize];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        n
    }

    /// Unit normal of a face (Newell's method, robust for non-planar n-gons).
    #[must_use]
    pub fn face_normal(&self, face: usize) -> [f32; 3] {
        normalize(self.newell(&self.faces[face]))
    }

    /// Area of a face.
    #[must_use]
    pub fn face_area(&self, face: usize) -> f32 {
        length(self.newell(&self.faces[face])) * 0.5
    }

    /// Vertex average of a face.
    #[must_use]
    pub fn face_centroid(&self, face: usize) -> [f32; 3] {
        let f = &self.faces[face];
        let sum = f
            .iter()
            .fold([0.0; 3], |acc, &v| add(acc, self.positions[v as usize]));
        scale(sum, 1.0 / f.len() as f32)
    }

    /// Area-weighted unit vertex normals.
    #[must_use]
    pub fn vertex_normals(&self) -> Vec<[f32; 3]> {
        let mut out = vec![[0.0_f32; 3]; self.positions.len()];
        for f in &self.faces {
            let n = self.newell(f);
            for &v in f {
                out[v as usize] = add(out[v as usize], n);
            }
        }
        out.into_iter().map(normalize).collect()
    }

    /// One-ring vertex neighbours (sorted, unique).
    #[must_use]
    pub fn neighbors(&self) -> Vec<Vec<u32>> {
        let mut out = vec![Vec::new(); self.positions.len()];
        for f in &self.faces {
            for i in 0..f.len() {
                let (a, b) = (f[i], f[(i + 1) % f.len()]);
                out[a as usize].push(b);
                out[b as usize].push(a);
            }
        }
        for n in &mut out {
            n.sort_unstable();
            n.dedup();
        }
        out
    }

    fn edge_faces(&self) -> HashMap<EdgeKey, Vec<usize>> {
        let mut map: HashMap<EdgeKey, Vec<usize>> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                map.entry(key(f[i], f[(i + 1) % f.len()]))
                    .or_default()
                    .push(fi);
            }
        }
        map
    }

    /// Edge counts and manifold diagnostics.
    #[must_use]
    pub fn topology(&self) -> Topology {
        let edges = self.edge_faces();
        let used: HashSet<u32> = self.faces.iter().flatten().copied().collect();
        let boundary_edges = edges.values().filter(|f| f.len() == 1).count();
        let non_manifold_edges = edges.values().filter(|f| f.len() > 2).count();
        Topology {
            vertices: used.len(),
            edges: edges.len(),
            faces: self.faces.len(),
            boundary_edges,
            non_manifold_edges,
            euler: used.len() as i64 - edges.len() as i64 + self.faces.len() as i64,
        }
    }

    /// Signed enclosed volume (positive for outward-facing closed meshes).
    #[must_use]
    pub fn signed_volume(&self) -> f32 {
        let mut v = 0.0;
        for f in &self.faces {
            let a = self.positions[f[0] as usize];
            for i in 1..f.len() - 1 {
                let b = self.positions[f[i] as usize];
                let c = self.positions[f[i + 1] as usize];
                v += dot(a, cross(b, c));
            }
        }
        v / 6.0
    }

    /// Axis-aligned bounds `(min, max)`; `None` for an empty mesh.
    #[must_use]
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let first = *self.positions.first()?;
        Some(self.positions.iter().fold((first, first), |(lo, hi), p| {
            (
                [lo[0].min(p[0]), lo[1].min(p[1]), lo[2].min(p[2])],
                [hi[0].max(p[0]), hi[1].max(p[1]), hi[2].max(p[2])],
            )
        }))
    }

    /// Faces whose normal is within `max_angle_deg` of `direction`.
    #[must_use]
    pub fn faces_facing(&self, direction: [f32; 3], max_angle_deg: f32) -> Vec<usize> {
        let d = normalize(direction);
        let min_dot = max_angle_deg.clamp(0.0, 180.0).to_radians().cos();
        (0..self.faces.len())
            .filter(|&f| dot(self.face_normal(f), d) >= min_dot - 1e-6)
            .collect()
    }

    /// Apply an affine matrix to every vertex.
    pub fn transform(&mut self, m: &Mat4) {
        for p in &mut self.positions {
            *p = m.transform_point(Vec3::from_array(*p)).to_array();
        }
    }

    fn check_faces(&self, faces: &[usize]) -> Result<Vec<bool>, PolyError> {
        let mut sel = vec![false; self.faces.len()];
        for &f in faces {
            *sel.get_mut(f)
                .ok_or_else(|| invalid(format!("face {f} out of range")))? = true;
        }
        if faces.is_empty() {
            return Err(invalid("empty face selection"));
        }
        Ok(sel)
    }

    /// Extrude a connected or disjoint face region along its averaged
    /// normals. The selected faces become the moved caps; every border edge
    /// of the selection gets a side quad. Returns the new side faces.
    ///
    /// # Errors
    /// [`PolyError`] for an invalid selection or distance.
    pub fn extrude_region(
        &mut self,
        faces: &[usize],
        distance: f32,
    ) -> Result<Vec<usize>, PolyError> {
        if !distance.is_finite() {
            return Err(invalid("extrude distance must be finite"));
        }
        let sel = self.check_faces(faces)?;
        let mut sel_edges: HashMap<EdgeKey, usize> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate().filter(|(fi, _)| sel[*fi]) {
            let _ = fi;
            for i in 0..f.len() {
                *sel_edges
                    .entry(key(f[i], f[(i + 1) % f.len()]))
                    .or_default() += 1;
            }
        }
        let mut offset = vec![[0.0_f32; 3]; self.positions.len()];
        let mut in_sel = vec![false; self.positions.len()];
        let mut in_unsel = vec![false; self.positions.len()];
        let mut on_border = vec![false; self.positions.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            if sel[fi] {
                let n = self.face_normal(fi);
                for i in 0..f.len() {
                    let v = f[i] as usize;
                    in_sel[v] = true;
                    offset[v] = add(offset[v], n);
                    if sel_edges[&key(f[i], f[(i + 1) % f.len()])] == 1 {
                        on_border[v] = true;
                        on_border[f[(i + 1) % f.len()] as usize] = true;
                    }
                }
            } else {
                for &v in f {
                    in_unsel[v as usize] = true;
                }
            }
        }
        let mut new_index: Vec<u32> = (0..self.positions.len() as u32).collect();
        for v in 0..self.positions.len() {
            if !in_sel[v] {
                continue;
            }
            let moved = add(self.positions[v], scale(normalize(offset[v]), distance));
            if in_unsel[v] || on_border[v] {
                new_index[v] = self.positions.len() as u32;
                self.positions.push(moved);
            } else {
                self.positions[v] = moved;
            }
        }
        if self.positions.len() > MAX_POLY_VERTICES {
            return Err(PolyError::TooLarge("extruded vertices"));
        }
        let mut sides = Vec::new();
        let count = self.faces.len();
        for fi in 0..count {
            if !sel[fi] {
                continue;
            }
            let f = self.faces[fi].clone();
            for i in 0..f.len() {
                let (a, b) = (f[i], f[(i + 1) % f.len()]);
                if sel_edges[&key(a, b)] == 1 {
                    sides.push(self.faces.len());
                    self.faces
                        .push(vec![a, b, new_index[b as usize], new_index[a as usize]]);
                }
            }
            self.faces[fi] = f.iter().map(|&v| new_index[v as usize]).collect();
        }
        if self.faces.len() > MAX_POLY_FACES {
            return Err(PolyError::TooLarge("extruded faces"));
        }
        Ok(sides)
    }

    /// Inset each selected face individually: the face shrinks towards its
    /// centroid by `amount` (`0..1`) and a ring of quads fills the gap.
    /// Returns the inner faces (same indices as the selection).
    ///
    /// # Errors
    /// [`PolyError`] for an invalid selection or amount.
    pub fn inset_faces(&mut self, faces: &[usize], amount: f32) -> Result<Vec<usize>, PolyError> {
        if !(amount > 0.0 && amount < 1.0) {
            return Err(invalid("inset amount must be in (0, 1)"));
        }
        let sel = self.check_faces(faces)?;
        let mut inner = Vec::new();
        for fi in 0..sel.len() {
            if !sel[fi] {
                continue;
            }
            let f = self.faces[fi].clone();
            let c = self.face_centroid(fi);
            let base = self.positions.len() as u32;
            for &v in &f {
                let p = self.positions[v as usize];
                self.positions.push(add(p, scale(sub(c, p), amount)));
            }
            let k = f.len() as u32;
            for i in 0..k {
                let j = (i + 1) % k;
                self.faces
                    .push(vec![f[i as usize], f[j as usize], base + j, base + i]);
            }
            self.faces[fi] = (0..k).map(|i| base + i).collect();
            inner.push(fi);
        }
        if self.positions.len() > MAX_POLY_VERTICES || self.faces.len() > MAX_POLY_FACES {
            return Err(PolyError::TooLarge("inset result"));
        }
        Ok(inner)
    }

    /// One level of Catmull-Clark subdivision (all-quad output). Open
    /// borders use the standard crease rule; non-manifold vertices stay put.
    ///
    /// # Errors
    /// [`PolyError`] when the result would exceed the limits.
    pub fn catmull_clark(&self) -> Result<Self, PolyError> {
        self.validate()?;
        let mut edge_index: HashMap<EdgeKey, usize> = HashMap::new();
        let mut edges: Vec<EdgeKey> = Vec::new();
        let mut edge_faces: Vec<Vec<usize>> = Vec::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                let k = key(f[i], f[(i + 1) % f.len()]);
                let e = *edge_index.entry(k).or_insert_with(|| {
                    edges.push(k);
                    edge_faces.push(Vec::new());
                    edges.len() - 1
                });
                edge_faces[e].push(fi);
            }
        }
        let (nv, ne, nf) = (self.positions.len(), edges.len(), self.faces.len());
        let quads: usize = self.faces.iter().map(Vec::len).sum();
        if nv + ne + nf > MAX_POLY_VERTICES {
            return Err(PolyError::TooLarge("subdivided vertices"));
        }
        if quads > MAX_POLY_FACES {
            return Err(PolyError::TooLarge("subdivided faces"));
        }
        let face_points: Vec<[f32; 3]> = (0..nf).map(|f| self.face_centroid(f)).collect();
        let mid = |e: EdgeKey| {
            scale(
                add(self.positions[e.0 as usize], self.positions[e.1 as usize]),
                0.5,
            )
        };
        let edge_points: Vec<[f32; 3]> = edges
            .iter()
            .zip(&edge_faces)
            .map(|(&e, fs)| {
                if fs.len() == 2 {
                    scale(
                        add(
                            add(self.positions[e.0 as usize], self.positions[e.1 as usize]),
                            add(face_points[fs[0]], face_points[fs[1]]),
                        ),
                        0.25,
                    )
                } else {
                    mid(e)
                }
            })
            .collect();
        let mut v_edges: Vec<Vec<usize>> = vec![Vec::new(); nv];
        for (e, &(a, b)) in edges.iter().enumerate() {
            v_edges[a as usize].push(e);
            v_edges[b as usize].push(e);
        }
        let mut v_faces: Vec<Vec<usize>> = vec![Vec::new(); nv];
        for (fi, f) in self.faces.iter().enumerate() {
            for &v in f {
                v_faces[v as usize].push(fi);
            }
        }
        let mut positions = Vec::with_capacity(nv + ne + nf);
        for v in 0..nv {
            let p = self.positions[v];
            let es = &v_edges[v];
            if es.is_empty() {
                positions.push(p);
                continue;
            }
            if es.iter().any(|&e| edge_faces[e].len() > 2) {
                positions.push(p);
                continue;
            }
            let border: Vec<usize> = es
                .iter()
                .copied()
                .filter(|&e| edge_faces[e].len() == 1)
                .collect();
            match border.len() {
                0 => {
                    let n = v_faces[v].len() as f32;
                    let q = scale(
                        v_faces[v]
                            .iter()
                            .fold([0.0; 3], |acc, &f| add(acc, face_points[f])),
                        1.0 / n,
                    );
                    let r = scale(
                        es.iter().fold([0.0; 3], |acc, &e| add(acc, mid(edges[e]))),
                        1.0 / es.len() as f32,
                    );
                    positions.push(scale(
                        add(add(q, scale(r, 2.0)), scale(p, n - 3.0)),
                        1.0 / n,
                    ));
                }
                2 => {
                    let other = |e: usize| {
                        let (a, b) = edges[e];
                        self.positions[if a as usize == v { b } else { a } as usize]
                    };
                    positions.push(add(
                        scale(p, 0.75),
                        scale(add(other(border[0]), other(border[1])), 0.125),
                    ));
                }
                _ => positions.push(p),
            }
        }
        positions.extend_from_slice(&edge_points);
        positions.extend_from_slice(&face_points);
        let mut faces = Vec::with_capacity(quads);
        for (fi, f) in self.faces.iter().enumerate() {
            let k = f.len();
            let fp = (nv + ne + fi) as u32;
            for i in 0..k {
                let e_next = edge_index[&key(f[i], f[(i + 1) % k])];
                let e_prev = edge_index[&key(f[(i + k - 1) % k], f[i])];
                faces.push(vec![f[i], (nv + e_next) as u32, fp, (nv + e_prev) as u32]);
            }
        }
        Ok(Self { positions, faces })
    }

    /// Weld vertices closer than `distance`, drop collapsed faces and unused
    /// vertices. Returns how many vertices were removed.
    pub fn merge_by_distance(&mut self, distance: f32) -> usize {
        let before = self.positions.len();
        let (positions, remap) = weld_positions(&self.positions, distance.max(0.0));
        self.positions = positions;
        let faces = std::mem::take(&mut self.faces);
        for f in faces {
            let mut out: Vec<u32> = Vec::with_capacity(f.len());
            for v in f {
                let m = remap[v as usize];
                if out.last() != Some(&m) {
                    out.push(m);
                }
            }
            if out.len() > 1 && out.first() == out.last() {
                out.pop();
            }
            let unique: HashSet<u32> = out.iter().copied().collect();
            if out.len() >= 3 && unique.len() == out.len() {
                self.faces.push(out);
            }
        }
        self.compact();
        before - self.positions.len()
    }

    /// Remove vertices no face references.
    pub fn compact(&mut self) {
        let mut used = vec![u32::MAX; self.positions.len()];
        let mut positions = Vec::with_capacity(self.positions.len());
        for f in &mut self.faces {
            for v in f.iter_mut() {
                let slot = &mut used[*v as usize];
                if *slot == u32::MAX {
                    *slot = positions.len() as u32;
                    positions.push(self.positions[*v as usize]);
                }
                *v = *slot;
            }
        }
        self.positions = positions;
    }

    /// Bevel (chamfer) the vertices incident to `edges` (or all edges when
    /// `edges` is empty) by fractional offset `amount` in `(0.0, 0.5)`.
    /// Returns the indices of newly created bevel and corner faces.
    ///
    /// # Errors
    /// [`PolyError`] when `amount` is outside `(0.0, 0.5)` or the mesh is invalid.
    pub fn bevel_edges(&mut self, edges: &[[u32; 2]], amount: f32) -> Result<Vec<usize>, PolyError> {
        if !(amount > 0.0 && amount < 0.5 && amount.is_finite()) {
            return Err(invalid("bevel amount must be in (0.0, 0.5)"));
        }
        self.validate()?;
        let mut v_bevel: HashSet<u32> = HashSet::new();
        if edges.is_empty() {
            for f in &self.faces {
                v_bevel.extend(f.iter().copied());
            }
        } else {
            for &[a, b] in edges {
                if a as usize >= self.positions.len() || b as usize >= self.positions.len() || a == b {
                    return Err(invalid("bevel edge references an invalid vertex"));
                }
                v_bevel.insert(a);
                v_bevel.insert(b);
            }
        }
        if v_bevel.is_empty() {
            return Err(invalid("no vertices selected for bevel"));
        }

        // Map directed half-edges (u, v) -> face index, and corner (fi, v) -> new vertex index.
        let mut half_edge_face: HashMap<(u32, u32), usize> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                let u = f[i];
                let v = f[(i + 1) % f.len()];
                if half_edge_face.insert((u, v), fi).is_some() {
                    return Err(invalid("bevel requires a manifold mesh with consistent winding"));
                }
            }
        }

        let mut corner_map: HashMap<(usize, u32), u32> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            if !f.iter().any(|v| v_bevel.contains(v)) {
                continue;
            }
            let c = scale(
                f.iter().fold([0.0; 3], |acc, &v| add(acc, self.positions[v as usize])),
                1.0 / f.len() as f32,
            );
            for &v in f {
                if v_bevel.contains(&v) {
                    let p = self.positions[v as usize];
                    let np = add(p, scale(sub(c, p), amount));
                    let idx = self.positions.len() as u32;
                    self.positions.push(np);
                    corner_map.insert((fi, v), idx);
                }
            }
        }

        // Collect undirected edges before mutating face loops.
        let mut undirected_edges: Vec<(u32, u32)> = Vec::new();
        let mut seen_edges: HashSet<EdgeKey> = HashSet::new();
        for f in &self.faces {
            for i in 0..f.len() {
                let u = f[i];
                let v = f[(i + 1) % f.len()];
                if seen_edges.insert(key(u, v)) {
                    undirected_edges.push((u, v));
                }
            }
        }

        // Record incoming/outgoing neighbors per (face, vertex) for corner polygon walking.
        let mut incoming_edge: HashMap<(usize, u32), u32> = HashMap::new();
        let mut outgoing_face: HashMap<(u32, u32), usize> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                let prev = f[(i + f.len() - 1) % f.len()];
                let curr = f[i];
                let next = f[(i + 1) % f.len()];
                incoming_edge.insert((fi, curr), prev);
                outgoing_face.insert((curr, next), fi);
            }
        }

        // Update original faces with corner vertices.
        for (fi, f) in self.faces.iter_mut().enumerate() {
            for v in f.iter_mut() {
                if let Some(&cv) = corner_map.get(&(fi, *v)) {
                    *v = cv;
                }
            }
        }

        let mut new_faces: Vec<usize> = Vec::new();

        // Add edge bevel quads / transition triangles.
        for (u, v) in undirected_edges {
            let u_in = v_bevel.contains(&u);
            let v_in = v_bevel.contains(&v);
            if !u_in && !v_in {
                continue;
            }
            let (Some(&f_left), Some(&f_right)) =
                (half_edge_face.get(&(u, v)), half_edge_face.get(&(v, u)))
            else {
                continue;
            };
            match (u_in, v_in) {
                (true, true) => {
                    let cl_u = corner_map[&(f_left, u)];
                    let cl_v = corner_map[&(f_left, v)];
                    let cr_u = corner_map[&(f_right, u)];
                    let cr_v = corner_map[&(f_right, v)];
                    new_faces.push(self.faces.len());
                    self.faces.push(vec![cl_v, cl_u, cr_u, cr_v]);
                }
                (true, false) => {
                    let cl_u = corner_map[&(f_left, u)];
                    let cr_u = corner_map[&(f_right, u)];
                    new_faces.push(self.faces.len());
                    self.faces.push(vec![v, cl_u, cr_u]);
                }
                (false, true) => {
                    let cl_v = corner_map[&(f_left, v)];
                    let cr_v = corner_map[&(f_right, v)];
                    new_faces.push(self.faces.len());
                    self.faces.push(vec![u, cr_v, cl_v]);
                }
                (false, false) => {}
            }
        }

        // Add vertex corner faces for interior beveled vertices.
        let mut sorted_verts: Vec<u32> = v_bevel.into_iter().collect();
        sorted_verts.sort_unstable();
        for u in sorted_verts {
            let incident: Vec<usize> = corner_map
                .keys()
                .filter_map(|&(fi, v)| (v == u).then_some(fi))
                .collect();
            if incident.len() < 3 {
                continue;
            }
            let start_f = *incident.iter().min().unwrap();
            let mut ring = Vec::with_capacity(incident.len());
            let mut curr_f = start_f;
            let mut closed = false;
            for _ in 0..incident.len() {
                ring.push(corner_map[&(curr_f, u)]);
                let Some(&v_in) = incoming_edge.get(&(curr_f, u)) else {
                    break;
                };
                let Some(&next_f) = outgoing_face.get(&(u, v_in)) else {
                    break;
                };
                if next_f == start_f {
                    closed = true;
                    break;
                }
                curr_f = next_f;
            }
            if closed && ring.len() >= 3 {
                new_faces.push(self.faces.len());
                self.faces.push(ring);
            }
        }

        self.compact();
        Ok(new_faces)
    }

    /// Insert an edge loop across the quad strip containing `(face_hint, edge_local)`
    /// at relative position `factor` in `(0.0, 1.0)`. Returns the newly created
    /// vertex indices along the cut.
    ///
    /// # Errors
    /// [`PolyError`] when `face_hint` is out of range, not a quad, or `factor` is invalid.
    pub fn loop_cut(
        &mut self,
        face_hint: usize,
        edge_local: usize,
        factor: f32,
    ) -> Result<Vec<u32>, PolyError> {
        if !(factor > 0.0 && factor < 1.0 && factor.is_finite()) {
            return Err(invalid("loop cut factor must be in (0.0, 1.0)"));
        }
        self.validate()?;
        let Some(f0) = self.faces.get(face_hint) else {
            return Err(invalid("face_hint out of range"));
        };
        if f0.len() != 4 {
            return Err(invalid("loop_cut requires a quad starting face"));
        }
        let e0 = edge_local % 4;
        let u0 = f0[e0];
        let v0 = f0[(e0 + 1) % 4];

        // Map undirected edges to incident faces.
        let mut edge_faces: HashMap<EdgeKey, Vec<usize>> = HashMap::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for i in 0..f.len() {
                edge_faces
                    .entry(key(f[i], f[(i + 1) % f.len()]))
                    .or_default()
                    .push(fi);
            }
        }

        // Walk the quad ring in both directions from `(u0, v0)`, keeping directed orientation `(a, b)`
        // so `factor` interpolates from rail A to rail B consistently.
        let mut split_vertex: HashMap<EdgeKey, u32> = HashMap::new();
        let mut created: Vec<u32> = Vec::new();

        let k0 = key(u0, v0);
        let pa0 = self.positions[u0 as usize];
        let pb0 = self.positions[v0 as usize];
        let p0 = add(pa0, scale(sub(pb0, pa0), factor));
        let idx0 = self.positions.len() as u32;
        self.positions.push(p0);
        split_vertex.insert(k0, idx0);
        created.push(idx0);

        // Walk across quads starting at `start_face` with entry directed edge `(ea, eb)`.
        for (start_f, start_edge) in [
            (Some(face_hint), (u0, v0)),
            (
                edge_faces
                    .get(&key(u0, v0))
                    .and_then(|fs| fs.iter().copied().find(|&fi| fi != face_hint)),
                (u0, v0),
            ),
        ] {
            let Some(mut curr_f) = start_f else { continue };
            let mut curr_edge = start_edge;
            loop {
                let f = &self.faces[curr_f];
                if f.len() != 4 {
                    break;
                }
                // Find local index of `curr_edge` (either forward or reverse).
                let Some(local_idx) = (0..4).find(|&i| {
                    key(f[i], f[(i + 1) % 4]) == key(curr_edge.0, curr_edge.1)
                }) else {
                    break;
                };
                let forward = f[local_idx] == curr_edge.0;
                let opp_idx = (local_idx + 2) % 4;
                // Opposite edge in the same rail direction (rail A -> rail B):
                // In quad [v0, v1, v2, v3], edge 0 is v0->v1 and opposite edge 2 is v2->v3,
                // so the parallel rail direction to v0->v1 is v3->v2.
                let opp_directed = if forward {
                    (f[(opp_idx + 1) % 4], f[opp_idx])
                } else {
                    (f[opp_idx], f[(opp_idx + 1) % 4])
                };
                let opp_key = key(opp_directed.0, opp_directed.1);
                if split_vertex.contains_key(&opp_key) {
                    break;
                }
                let pa = self.positions[opp_directed.0 as usize];
                let pb = self.positions[opp_directed.1 as usize];
                let p = add(pa, scale(sub(pb, pa), factor));
                let idx = self.positions.len() as u32;
                self.positions.push(p);
                split_vertex.insert(opp_key, idx);
                created.push(idx);
                let next_f = edge_faces
                    .get(&opp_key)
                    .and_then(|fs| fs.iter().copied().find(|&fi| fi != curr_f));
                let Some(nf) = next_f else { break };
                curr_f = nf;
                curr_edge = opp_directed;
            }
        }

        // Rebuild faces: quads with two opposite split edges become two quads;
        // any other face touching a split edge gets the midpoint vertex inserted along that edge.
        let old_faces = std::mem::take(&mut self.faces);
        for f in old_faces {
            if f.len() == 4 {
                let s0 = split_vertex.get(&key(f[0], f[1])).copied();
                let s1 = split_vertex.get(&key(f[1], f[2])).copied();
                let s2 = split_vertex.get(&key(f[2], f[3])).copied();
                let s3 = split_vertex.get(&key(f[3], f[0])).copied();
                if let (Some(m0), None, Some(m2), None) = (s0, s1, s2, s3) {
                    self.faces.push(vec![f[0], m0, m2, f[3]]);
                    self.faces.push(vec![m0, f[1], f[2], m2]);
                    continue;
                }
                if let (None, Some(m1), None, Some(m3)) = (s0, s1, s2, s3) {
                    self.faces.push(vec![f[0], f[1], m1, m3]);
                    self.faces.push(vec![m3, m1, f[2], f[3]]);
                    continue;
                }
            }
            let mut expanded = Vec::with_capacity(f.len() + 2);
            for i in 0..f.len() {
                let u = f[i];
                let v = f[(i + 1) % f.len()];
                expanded.push(u);
                if let Some(&m) = split_vertex.get(&key(u, v)) {
                    expanded.push(m);
                }
            }
            self.faces.push(expanded);
        }
        Ok(created)
    }

    /// Bridge two faces `face_a` and `face_b` of equal vertex count with a tube
    /// of quad faces, removing the two cap faces. Returns the indices of the
    /// newly created bridge quads.
    ///
    /// # Errors
    /// [`PolyError`] when the faces are out of range, identical, share vertices,
    /// or have different vertex counts.
    pub fn bridge_faces(&mut self, face_a: usize, face_b: usize) -> Result<Vec<usize>, PolyError> {
        self.validate()?;
        if face_a == face_b || face_a >= self.faces.len() || face_b >= self.faces.len() {
            return Err(invalid("bridge_faces requires two distinct valid face indices"));
        }
        let loop_a = self.faces[face_a].clone();
        let loop_b = self.faces[face_b].clone();
        let k = loop_a.len();
        if k != loop_b.len() || k < 3 {
            return Err(invalid("bridge_faces requires two faces with the same vertex count"));
        }
        if loop_a.iter().any(|v| loop_b.contains(v)) {
            return Err(invalid("bridge_faces faces must not share vertices"));
        }

        // Reverse loop_b so its cyclic orientation around the bridge axis matches loop_a,
        // then pick the cyclic shift that minimizes total edge length.
        let b_rev: Vec<u32> = loop_b.into_iter().rev().collect();
        let mut best_shift = 0usize;
        let mut best_cost = f32::MAX;
        for shift in 0..k {
            let cost: f32 = (0..k)
                .map(|i| {
                    let pa = self.positions[loop_a[i] as usize];
                    let pb = self.positions[b_rev[(i + shift) % k] as usize];
                    let d = sub(pa, pb);
                    dot(d, d)
                })
                .sum();
            if cost < best_cost {
                best_cost = cost;
                best_shift = shift;
            }
        }

        let b_aligned: Vec<u32> = (0..k).map(|i| b_rev[(i + best_shift) % k]).collect();

        // Remove the two cap faces (higher index first).
        let (hi, lo) = if face_a > face_b {
            (face_a, face_b)
        } else {
            (face_b, face_a)
        };
        self.faces.remove(hi);
        self.faces.remove(lo);

        let mut bridge_indices = Vec::with_capacity(k);
        for i in 0..k {
            let next = (i + 1) % k;
            bridge_indices.push(self.faces.len());
            self.faces.push(vec![
                loop_a[i],
                loop_a[next],
                b_aligned[next],
                b_aligned[i],
            ]);
        }
        Ok(bridge_indices)
    }

    /// Triangulate a single face loop into `out` using 2D ear-clipping projected
    /// onto the face's Newell normal plane (with fan fallback for degenerate faces).
    fn triangulate_face(&self, f: &[u32], out: &mut Vec<u32>) {
        if f.len() < 3 {
            return;
        }
        if f.len() == 3 {
            out.extend_from_slice(f);
            return;
        }
        // Compute Newell normal.
        let mut n = [0.0_f32; 3];
        for i in 0..f.len() {
            let a = self.positions[f[i] as usize];
            let b = self.positions[f[(i + 1) % f.len()] as usize];
            n[0] += (a[1] - b[1]) * (a[2] + b[2]);
            n[1] += (a[2] - b[2]) * (a[0] + b[0]);
            n[2] += (a[0] - b[0]) * (a[1] + b[1]);
        }
        let n = normalize(n);
        if length(n) < 1e-6 {
            for i in 1..f.len() - 1 {
                out.extend_from_slice(&[f[0], f[i], f[i + 1]]);
            }
            return;
        }
        // Check if polygon is already strictly convex: if so, fast fan triangulation is exact.
        let ref_axis = if n[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let u_axis = normalize(cross(n, ref_axis));
        let v_axis = cross(n, u_axis);
        let project = |vid: u32| -> [f32; 2] {
            let p = self.positions[vid as usize];
            [dot(p, u_axis), dot(p, v_axis)]
        };
        let cross2 = |a: [f32; 2], b: [f32; 2], c: [f32; 2]| -> f32 {
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        };
        let point_in_tri = |p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]| -> bool {
            let c1 = cross2(a, b, p);
            let c2 = cross2(b, c, p);
            let c3 = cross2(c, a, p);
            c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6
        };

        let mut ring: Vec<u32> = f.to_vec();
        let mut guard = ring.len() * 3;
        while ring.len() > 3 && guard > 0 {
            guard -= 1;
            let m = ring.len();
            let mut clipped = false;
            for i in 0..m {
                let prev = ring[(i + m - 1) % m];
                let curr = ring[i];
                let next = ring[(i + 1) % m];
                let (pa, pb, pc) = (project(prev), project(curr), project(next));
                if cross2(pa, pb, pc) <= 1e-7 {
                    continue;
                }
                let has_inside = (0..m).any(|j| {
                    let v = ring[j];
                    v != prev && v != curr && v != next && point_in_tri(project(v), pa, pb, pc)
                });
                if !has_inside {
                    out.extend_from_slice(&[prev, curr, next]);
                    ring.remove(i);
                    clipped = true;
                    break;
                }
            }
            if !clipped {
                break;
            }
        }
        if ring.len() == 3 {
            out.extend_from_slice(&ring);
        } else {
            for i in 1..ring.len() - 1 {
                out.extend_from_slice(&[ring[0], ring[i], ring[i + 1]]);
            }
        }
    }

    /// Ear-clipped triangulated index list (handles both convex and concave n-gons).
    #[must_use]
    pub fn triangulate(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for f in &self.faces {
            self.triangulate_face(f, &mut out);
        }
        out
    }

    /// Two-sided ray pick; `direction` need not be normalized.
    #[must_use]
    pub fn raycast(&self, origin: [f32; 3], direction: [f32; 3]) -> Option<RayHit> {
        let d = normalize(direction);
        if length(d) == 0.0 {
            return None;
        }
        let mut best: Option<RayHit> = None;
        for (fi, f) in self.faces.iter().enumerate() {
            let a = self.positions[f[0] as usize];
            for i in 1..f.len() - 1 {
                let b = self.positions[f[i] as usize];
                let c = self.positions[f[i + 1] as usize];
                let (e1, e2) = (sub(b, a), sub(c, a));
                let p = cross(d, e2);
                let det = dot(e1, p);
                if det.abs() < 1e-12 {
                    continue;
                }
                let inv = 1.0 / det;
                let s = sub(origin, a);
                let u = dot(s, p) * inv;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(d, q) * inv;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let t = dot(e2, q) * inv;
                if t > 1e-6 && best.is_none_or(|h| t < h.distance) {
                    best = Some(RayHit {
                        face: fi,
                        distance: t,
                        point: add(origin, scale(d, t)),
                    });
                }
            }
        }
        best
    }

    /// Convert to renderer geometry (positions are used as-is).
    #[must_use]
    pub fn to_geometry(&self, shading: Shading) -> Geometry {
        match shading {
            Shading::Smooth => Geometry::Mesh {
                positions: self.positions.clone(),
                normals: Some(self.vertex_normals()),
                indices: self.triangulate(),
            },
            Shading::Flat => {
                let mut positions = Vec::new();
                let mut normals = Vec::new();
                let mut indices = Vec::new();
                for (fi, f) in self.faces.iter().enumerate() {
                    let n = self.face_normal(fi);
                    let base = positions.len() as u32;
                    let mut local_ring = Vec::with_capacity(f.len());
                    for (k, &v) in f.iter().enumerate() {
                        positions.push(self.positions[v as usize]);
                        normals.push(n);
                        local_ring.push((v, base + k as u32));
                    }
                    let mut face_tris = Vec::new();
                    self.triangulate_face(f, &mut face_tris);
                    for vid in face_tris {
                        let mapped = local_ring
                            .iter()
                            .find_map(|&(orig, loc)| (orig == vid).then_some(loc))
                            .unwrap_or(base);
                        indices.push(mapped);
                    }
                }
                Geometry::Mesh {
                    positions,
                    normals: Some(normals),
                    indices,
                }
            }
        }
    }
}

/// Weld positions on a quantized grid of cell `eps`; returns the unique
/// positions and the old-to-new index map.
fn weld_positions(positions: &[[f32; 3]], eps: f32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let cell = if eps > 0.0 && eps.is_finite() {
        eps
    } else {
        0.0
    };
    let quant = |p: [f32; 3], o: [f32; 3]| -> [i64; 3] {
        if cell == 0.0 {
            return [
                i64::from(p[0].to_bits()),
                i64::from(p[1].to_bits()),
                i64::from(p[2].to_bits()),
            ];
        }
        [
            ((p[0] + o[0]) / cell).floor() as i64,
            ((p[1] + o[1]) / cell).floor() as i64,
            ((p[2] + o[2]) / cell).floor() as i64,
        ]
    };
    let mut map: HashMap<[i64; 3], u32> = HashMap::new();
    let mut out = Vec::new();
    let mut remap = Vec::with_capacity(positions.len());
    for &p in positions {
        let k = quant(p, [cell * 0.5; 3]);
        let found = if cell == 0.0 {
            map.get(&k).copied()
        } else {
            // Look in the 27 neighbouring cells for a point within `eps`.
            let mut hit = None;
            'outer: for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(&i) = map.get(&[k[0] + dx, k[1] + dy, k[2] + dz]) {
                            if length(sub(out[i as usize], p)) <= cell {
                                hit = Some(i);
                                break 'outer;
                            }
                        }
                    }
                }
            }
            hit
        };
        let idx = found.unwrap_or_else(|| {
            let i = out.len() as u32;
            out.push(p);
            map.entry(k).or_insert(i);
            i
        });
        remap.push(idx);
    }
    (out, remap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_is_closed_and_outward() {
        let c = PolyMesh::cube([2.0, 2.0, 2.0]);
        c.validate().unwrap();
        let t = c.topology();
        assert_eq!((t.vertices, t.edges, t.faces, t.euler), (8, 12, 6, 2));
        assert_eq!(t.boundary_edges, 0);
        assert!((c.signed_volume() - 8.0).abs() < 1e-5);
        assert_eq!(c.faces_facing([0.0, 1.0, 0.0], 1.0), vec![4]);
    }

    #[test]
    fn extrude_top_face_keeps_manifold_and_adds_volume() {
        let mut c = PolyMesh::cube([2.0, 2.0, 2.0]);
        let top = c.faces_facing([0.0, 1.0, 0.0], 1.0);
        let sides = c.extrude_region(&top, 1.0).unwrap();
        assert_eq!(sides.len(), 4);
        let t = c.topology();
        assert_eq!(
            (t.vertices, t.faces, t.boundary_edges, t.euler),
            (12, 10, 0, 2)
        );
        assert!((c.signed_volume() - 12.0).abs() < 1e-4);
        let (_, hi) = c.bounds().unwrap();
        assert!((hi[1] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn extrude_region_of_two_adjacent_faces() {
        let mut g = PolyMesh::grid([2.0, 1.0], [2, 1]).unwrap();
        let sides = g.extrude_region(&[0, 1], 0.5).unwrap();
        // Border of a 2x1 strip has 6 edges.
        assert_eq!(sides.len(), 6);
        assert_eq!(g.topology().non_manifold_edges, 0);
    }

    #[test]
    fn inset_then_extrude_makes_a_panel() {
        let mut c = PolyMesh::cube([2.0, 2.0, 2.0]);
        let inner = c.inset_faces(&[0], 0.25).unwrap();
        c.extrude_region(&inner, -0.2).unwrap();
        let t = c.topology();
        assert_eq!(t.boundary_edges, 0);
        assert_eq!(t.euler, 2);
        assert!(c.signed_volume() < 8.0);
    }

    #[test]
    fn catmull_clark_cube_counts_and_shrinks() {
        let c = PolyMesh::cube([2.0, 2.0, 2.0]);
        let s = c.catmull_clark().unwrap();
        let t = s.topology();
        assert_eq!((t.vertices, t.faces, t.edges, t.euler), (26, 24, 48, 2));
        let s2 = s.catmull_clark().unwrap();
        assert_eq!(s2.faces.len(), 96);
        // The limit surface lies strictly inside the control cage.
        let v = s2.signed_volume();
        assert!(
            v > 2.0 && v < s.signed_volume() && s.signed_volume() < 8.0,
            "{v}"
        );
    }

    #[test]
    fn catmull_clark_open_grid_keeps_border() {
        let g = PolyMesh::grid([2.0, 2.0], [2, 2]).unwrap();
        let s = g.catmull_clark().unwrap();
        assert_eq!(s.faces.len(), 16);
        assert_eq!(s.topology().boundary_edges, 16);
        // A flat grid stays flat; the crease rule keeps the border in place
        // along its own line (edge midpoints are exact).
        assert!(s.positions.iter().all(|p| p[1].abs() < 1e-6));
        assert!(
            s.positions
                .iter()
                .any(|p| (p[0] + 1.0).abs() < 1e-6 && p[2].abs() < 1e-6)
        );
    }

    #[test]
    fn quad_sphere_is_round() {
        let s = PolyMesh::quad_sphere(1.5, 3).unwrap();
        assert_eq!(s.faces.len(), 6 * 64);
        for p in &s.positions {
            assert!((length(*p) - 1.5).abs() < 1e-4);
        }
        assert_eq!(s.topology().euler, 2);
    }

    #[test]
    fn from_geometry_welds_seams() {
        let m = PolyMesh::from_geometry(&Geometry::Sphere {
            radius: 1.0,
            width_segments: 16,
            height_segments: 8,
        })
        .unwrap();
        let t = m.topology();
        assert_eq!(t.boundary_edges, 0, "{t:?}");
        assert_eq!(t.euler, 2);
    }

    #[test]
    fn merge_by_distance_and_raycast() {
        let mut c = PolyMesh::cube([2.0, 2.0, 2.0]);
        c.positions.push([1.0, 1.0, 1.0 + 1e-7]);
        c.faces[0][2] = 8;
        assert_eq!(c.merge_by_distance(1e-4), 1);
        assert_eq!(c.topology().boundary_edges, 0);
        let hit = c.raycast([0.0, 5.0, 0.0], [0.0, -1.0, 0.0]).unwrap();
        assert!((hit.distance - 4.0).abs() < 1e-5);
        assert!(dot(c.face_normal(hit.face), [0.0, 1.0, 0.0]) > 0.99);
    }

    #[test]
    fn flat_and_smooth_geometry_tessellate() {
        let c = PolyMesh::cube([1.0, 1.0, 1.0]);
        for s in [Shading::Smooth, Shading::Flat] {
            let data = c.to_geometry(s).tessellate().unwrap();
            assert_eq!(data.triangle_count(), 12);
        }
    }

    #[test]
    fn invalid_selection_is_rejected() {
        let mut c = PolyMesh::cube([1.0, 1.0, 1.0]);
        assert!(c.extrude_region(&[99], 1.0).is_err());
        assert!(c.inset_faces(&[0], 1.5).is_err());
        assert!(PolyMesh::quad_sphere(1.0, 9).is_err());
    }

    #[test]
    fn bevel_all_cube_edges_stays_closed_manifold() {
        let mut c = PolyMesh::cube([2.0, 2.0, 2.0]);
        let created = c.bevel_edges(&[], 0.2).unwrap();
        assert_eq!(created.len(), 20); // 12 edge quads + 8 corner triangles
        let t = c.topology();
        assert_eq!((t.vertices, t.edges, t.faces, t.euler), (24, 48, 26, 2));
        assert_eq!(t.boundary_edges, 0);
        assert_eq!(t.non_manifold_edges, 0);
    }

    #[test]
    fn loop_cut_splits_cube_ring_into_manifold_quads() {
        let mut c = PolyMesh::cube([2.0, 2.0, 2.0]);
        let cut_verts = c.loop_cut(0, 0, 0.5).unwrap();
        assert_eq!(cut_verts.len(), 4);
        let t = c.topology();
        assert_eq!((t.vertices, t.faces, t.boundary_edges, t.euler), (12, 10, 0, 2));
        assert_eq!(t.non_manifold_edges, 0);
    }

    #[test]
    fn bridge_faces_punches_genus_one_tunnel() {
        let mut c = PolyMesh::cube([2.0, 2.0, 2.0]);
        let top = c.faces_facing([0.0, 1.0, 0.0], 1.0)[0];
        let bot = c.faces_facing([0.0, -1.0, 0.0], 1.0)[0];
        let inner = c.inset_faces(&[top, bot], 0.4).unwrap();
        let bridged = c.bridge_faces(inner[0], inner[1]).unwrap();
        assert_eq!(bridged.len(), 4);
        let t = c.topology();
        assert_eq!(t.boundary_edges, 0);
        assert_eq!(t.non_manifold_edges, 0);
        assert_eq!(t.euler, 0); // Genus-1 torus topology: V - E + F = 0
    }

    #[test]
    fn ear_clipping_triangulates_concave_ngon() {
        // Concave L-shaped 6-gon in the XZ plane facing +Y.
        let m = PolyMesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 2.0],
                [2.0, 0.0, 2.0],
                [2.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0],
            ],
            faces: vec![vec![0, 1, 2, 3, 4, 5]],
        };
        let tris = m.triangulate();
        assert_eq!(tris.len(), 12); // 4 triangles * 3 indices
        // Total area of the L-shape is 3.0; sum of oriented triangle areas must equal 3.0.
        let mut area = 0.0_f32;
        for tri in tris.chunks_exact(3) {
            let a = m.positions[tri[0] as usize];
            let b = m.positions[tri[1] as usize];
            let c = m.positions[tri[2] as usize];
            let n = cross(sub(b, a), sub(c, a));
            assert!(n[1] > 0.0, "triangle flipped: {tri:?}");
            area += 0.5 * n[1];
        }
        assert!((area - 3.0).abs() < 1e-5, "area={area}");
    }
}
