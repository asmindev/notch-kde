struct Material {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    params: vec4<f32>,
};

struct SceneUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dirs: array<vec4<f32>, 3>,
    light_colors: array<vec4<f32>, 3>,
    ambient: vec4<f32>,
    node_matrices: array<mat4x4<f32>, 24>,
    node_normal_matrices: array<mat4x4<f32>, 24>,
    materials: array<Material, 8>,
};

@group(0) @binding(0) var<uniform> scene: SceneUniform;

struct SceneVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) node_id: u32,
    @location(3) material_id: u32,
};

struct SceneVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) material_id: u32,
};

@vertex
fn vs_scene(input: SceneVertexInput) -> SceneVertexOutput {
    let world = scene.node_matrices[input.node_id];
    let normal_matrix = scene.node_normal_matrices[input.node_id];
    let world_position = world * vec4<f32>(input.position, 1.0);

    var output: SceneVertexOutput;
    output.position = scene.view_proj * world_position;
    output.world_position = world_position.xyz;
    output.normal = normalize((normal_matrix * vec4<f32>(input.normal, 0.0)).xyz);
    output.material_id = input.material_id;
    return output;
}

@fragment
fn fs_scene(input: SceneVertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let material = scene.materials[input.material_id];
    var normal = normalize(input.normal);
    if (!front_facing) {
        normal = -normal;
    }

    let view_dir = normalize(scene.camera_pos.xyz - input.world_position);
    let base = material.base_color.rgb;
    let metallic = clamp(material.params.x, 0.0, 1.0);
    let roughness = clamp(material.params.y, 0.04, 1.0);
    let clearcoat = clamp(material.params.z, 0.0, 1.0);

    var color = base * scene.ambient.rgb;
    for (var i = 0u; i < 3u; i = i + 1u) {
        let light_dir = normalize(scene.light_dirs[i].xyz);
        let half_dir = normalize(light_dir + view_dir);
        let n_dot_l = max(dot(normal, light_dir), 0.0);
        let n_dot_h = max(dot(normal, half_dir), 0.0);
        let specular_power = mix(120.0, 8.0, roughness);
        let specular = pow(n_dot_h, specular_power) * mix(0.15, 1.0, metallic) * (0.30 + clearcoat);
        color += (base * n_dot_l + vec3<f32>(specular)) * scene.light_colors[i].rgb;
    }
    color += material.emissive.rgb;

    color = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14);
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
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
