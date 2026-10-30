struct BodyUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    body_color: vec4<f32>,
    eye_color: vec4<f32>,
    params: vec4<f32>,
    eye_centers: array<vec4<f32>, 2>,
    eye_inv: array<vec4<f32>, 2>,
    eye_extra: array<vec4<f32>, 2>,
    eye_style: array<vec4<f32>, 2>,
    notif: vec4<f32>,
    light_dirs: array<vec4<f32>, 3>,
    light_colors: array<vec4<f32>, 3>,
    ambient: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: BodyUniform;

struct BodyVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct BodyVertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_body(input: BodyVertexInput) -> BodyVertexOutput {
    var out: BodyVertexOutput;
    out.clip = u.view_proj * vec4<f32>(input.position, 1.0);
    out.world = input.position;
    out.normal = input.normal;
    return out;
}

fn eye_sdf(p: vec2<f32>, index: i32) -> f32 {
    let center = u.eye_centers[index].xy;
    let d = p - center;
    let open = max(u.eye_extra[index].z, 0.06);
    let squashed = vec2<f32>(d.x, d.y / open);
    let inv = u.eye_inv[index];
    let q = vec2<f32>(dot(inv.xy, squashed), dot(inv.zw, squashed));
    let hw = max(u.eye_extra[index].x, 0.0001);
    let hh = max(u.eye_extra[index].y, 0.0001);
    let r = min(hw, hh) * clamp(u.eye_style[index].x, 0.05, 1.0);
    let edge = vec2<f32>(abs(q.x) - (hw - r), abs(q.y) - (hh - r));
    return length(max(edge, vec2<f32>(0.0))) + min(max(edge.x, edge.y), 0.0) - r;
}

fn eye_mask(p: vec2<f32>, index: i32) -> f32 {
    if (u.eye_extra[index].w < 0.5) {
        return 0.0;
    }
    return 1.0 - smoothstep(-0.012, 0.020, eye_sdf(p, index));
}

@fragment
fn fs_body(input: BodyVertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let p = input.world.xy;
    if (u.notif.w > 0.5) {
        if (distance(p, u.notif.xy) < u.notif.z) {
            discard;
        }
    }

    var normal = normalize(input.normal);
    if (!front_facing) {
        normal = -normal;
    }

    let mask = max(eye_mask(p, 0), eye_mask(p, 1)) * u.params.y;
    if (u.params.z > 0.5) {
        if (mask < 0.01) {
            discard;
        }
        return vec4<f32>(1.0, 1.0, 1.0, mask);
    }
    var base = mix(u.body_color.rgb, u.eye_color.rgb, mask);
    let view_dir = normalize(u.camera_pos.xyz - input.world);

    var color = base * u.ambient.rgb;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let light_dir = normalize(u.light_dirs[i].xyz);
        let half_dir = normalize(light_dir + view_dir);
        let n_dot_l = max(dot(normal, light_dir), 0.0);
        let n_dot_h = max(dot(normal, half_dir), 0.0);
        let specular = pow(n_dot_h, 42.0) * 0.16;
        color += (base * n_dot_l + vec3<f32>(specular)) * u.light_colors[i].rgb;
    }

    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0) * 0.18;
    color += rim * u.body_color.rgb;
    color = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14);

    let alpha = u.body_color.a * u.params.x;
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}

struct OverlayVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct OverlayVertexOutput {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_overlay(input: OverlayVertexInput) -> OverlayVertexOutput {
    var out: OverlayVertexOutput;
    out.clip = vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_overlay(input: OverlayVertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

struct CompositeVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_composite(@builtin(vertex_index) vertex_index: u32) -> CompositeVertexOutput {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );

    var output: CompositeVertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}

@group(0) @binding(0) var source_texture: texture_2d<f32>;
@group(0) @binding(1) var source_sampler: sampler;

@fragment
fn fs_composite(input: CompositeVertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(source_texture, source_sampler, input.uv);
    let alpha = max(color.a, 0.0001);
    return vec4<f32>(color.rgb / alpha, color.a);
}
