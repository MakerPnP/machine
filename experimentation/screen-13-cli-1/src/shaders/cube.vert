#version 450

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec3 fragWorldPos;
layout(location = 2) out vec3 fragNormal;

layout(push_constant) uniform PushConstants {
    mat4 mvp;
    mat4 model;
    vec3 lightPos;
    float lightIntensity;
    vec3 lightColor;
} pc;

void main() {
    gl_Position = pc.mvp * vec4(inPosition, 1.0);

    // Pass world position for lighting calculations
    fragWorldPos = (pc.model * vec4(inPosition, 1.0)).xyz;

    // Calculate normal (for flat shading, we'll compute it in fragment shader)
    // For now, pass a placeholder - fragment shader will compute from derivatives
    fragNormal = vec3(0.0);

    fragColor = inColor;
}