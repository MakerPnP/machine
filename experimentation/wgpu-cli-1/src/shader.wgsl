// Vertex shader

struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    light_pos: vec3<f32>,
    light_color: vec3<f32>,
    object_color: vec3<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;

    let world_position = uniforms.model * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.world_normal = normalize((uniforms.model * vec4<f32>(model.normal, 0.0)).xyz);
    out.clip_position = uniforms.view_proj * world_position;

    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ambient lighting
    let ambient_strength = 0.2;
    let ambient = ambient_strength * uniforms.light_color;

    // Diffuse lighting
    let light_dir = normalize(uniforms.light_pos - in.world_position);
    let diff = max(dot(in.world_normal, light_dir), 0.0);
    let diffuse = diff * uniforms.light_color;

    // Specular lighting
    let view_dir = normalize(-in.world_position);
    let reflect_dir = reflect(-light_dir, in.world_normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let specular_strength = 0.5;
    let specular = specular_strength * spec * uniforms.light_color;

    // Combine lighting
    let result = (ambient + diffuse + specular) * uniforms.object_color;

    return vec4<f32>(result, 1.0);
}