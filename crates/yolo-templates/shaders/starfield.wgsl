// SPDX-License-Identifier: Apache-2.0
// Starfield background for GPU hosts (wgpu, Bevy). Original shader for yolo-templates.
//
// Draw one full-screen triangle with no vertex buffer. The host owns the clock
// (`StarfieldClock` in motion.rs): when the user pauses or the system asks for reduced
// motion, `time` stops advancing and the host stops requesting frames, so the last frame
// stays on screen.

struct Uniforms {
    resolution: vec2<f32>,
    time: f32,
    density: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    // Three vertices covering the viewport: (-1,-1), (3,-1), (-1,3).
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    var out: VsOut;
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q) * 43758.5453);
}

fn hash1(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(41.3, 289.1))) * 15731.743);
}

// One layer of stars on a grid of `cell` px, drifting at `speed` px per second.
fn layer(frag: vec2<f32>, cell: f32, speed: f32, size: f32) -> f32 {
    let p = (frag + vec2<f32>(u.time * speed, u.time * speed * 0.35)) / cell;
    let id = floor(p);
    let f = fract(p);
    if (hash1(id) > u.density) {
        return 0.0;
    }
    let center = hash2(id) * 0.8 + 0.1;
    let d = length((f - center) * cell);
    let twinkle = 0.65 + 0.35 * sin(u.time * (0.6 + hash1(id + 7.0) * 1.4) + hash1(id + 3.0) * 6.2831);
    return (1.0 - smoothstep(0.0, size, d)) * twinkle;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let frag = in.uv * u.resolution;
    // Deep field: a vertical gradient that stays dark enough for white text at 7:1.
    let t = in.uv.y;
    var color = mix(vec3<f32>(0.016, 0.02, 0.043), vec3<f32>(0.031, 0.035, 0.07), t);
    let far = layer(frag, 22.0, 1.5, 0.9);
    let mid = layer(frag, 48.0, 4.0, 1.3);
    let near = layer(frag, 96.0, 9.0, 1.8);
    let light = far * 0.45 + mid * 0.7 + near;
    color = color + vec3<f32>(0.86, 0.9, 1.0) * light;
    return vec4<f32>(min(color, vec3<f32>(1.0)), 1.0);
}
