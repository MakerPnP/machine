struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    light_pos: vec4<f32>,  // Changed from vec3 to vec4
    light_intensity: f32,
    // WGSL will insert padding here automatically
    light_color: vec4<f32>,  // Changed from vec3 to vec4
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_pos: vec3<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = uniforms.mvp * vec4<f32>(in.position, 1.0);
    out.world_pos = (uniforms.model * vec4<f32>(in.position, 1.0)).xyz;
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate normal using derivatives (flat shading)
    let dpdx = dpdx(in.world_pos);
    let dpdy = dpdy(in.world_pos);
    let normal = normalize(cross(dpdx, dpdy));

    // Calculate light direction
    let light_dir = normalize(uniforms.light_pos - in.world_pos);

    // Calculate distance for attenuation
    let distance = length(uniforms.light_pos - in.world_pos);
    let attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);

    // Diffuse lighting
    let diff = max(dot(normal, light_dir), 0.0);
    let diffuse = diff * uniforms.light_color * uniforms.light_intensity * attenuation;

    // Ambient lighting
    let ambient = 0.2 * in.color;

    // Combine lighting with object color
    let result = (ambient + diffuse) * in.color;

    return vec4<f32>(result, 1.0);
}
