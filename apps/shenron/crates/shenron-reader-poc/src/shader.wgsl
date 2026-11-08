// Fullscreen triangle + fragment that renders either the page texture
// (pan/zoom in image space) or a checkerboard placeholder.

struct Uniforms {
    // canvas rect in surface pixels: x, y, w, h
    rect: vec4<f32>,
    // pan in surface pixels (x, y), zoom, has_image (0/1)
    pan_zoom: vec4<f32>,
    // image size in texels (w, h), device pixel ratio, time
    image: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var page_tex: texture_2d<f32>;
@group(0) @binding(2) var page_smp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    // Covers the whole viewport with one triangle.
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(p[i], 0.0, 1.0);
    return out;
}

fn checker(p: vec2<f32>, cell: f32) -> f32 {
    let c = floor(p / cell);
    return f32((i32(c.x) + i32(c.y)) & 1);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let bg = vec3<f32>(0.055, 0.059, 0.071);
    let local = in.pos.xy - u.rect.xy;            // canvas-local pixels
    let center = u.rect.zw * 0.5;
    let zoom = max(u.pan_zoom.z, 0.01);
    let has_image = u.pan_zoom.w > 0.5;
    let img_size = u.image.xy;

    // image-space coordinate (texels)
    let q = (local - center - u.pan_zoom.xy) / zoom + img_size * 0.5;
    let inside = all(q >= vec2<f32>(0.0)) && all(q < img_size);

    if (!inside) {
        // subtle grid on the backdrop so pan is readable
        let g = checker(local - u.pan_zoom.xy, 24.0 * zoom);
        return vec4<f32>(mix(bg, bg * 1.25, g), 1.0);
    }

    if (has_image) {
        let uv = q / img_size;
        let c = textureSample(page_tex, page_smp, uv);
        return vec4<f32>(c.rgb, 1.0);
    }

    // placeholder page: light checkerboard with an amber frame
    let cell = 32.0 * zoom;
    let k = checker(q * zoom, cell);
    var col = mix(vec3<f32>(0.78, 0.77, 0.74), vec3<f32>(0.62, 0.61, 0.58), k);
    let edge = min(min(q.x, q.y), min(img_size.x - q.x, img_size.y - q.y)) * zoom;
    if (edge < 3.0) {
        col = vec3<f32>(0.949, 0.647, 0.231);
    }
    return vec4<f32>(col, 1.0);
}
