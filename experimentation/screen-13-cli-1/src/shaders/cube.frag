#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec3 fragWorldPos;
layout(location = 2) in vec3 fragNormal;

layout(location = 0) out vec4 outColor;

layout(push_constant) uniform PushConstants {
    mat4 mvp;
    mat4 model;
    vec3 lightPos;
    float lightIntensity;
    vec3 lightColor;
} pc;

void main() {
    // Calculate normal using derivatives (flat shading)
    vec3 normal = normalize(cross(dFdx(fragWorldPos), dFdy(fragWorldPos)));

    // Calculate light direction
    vec3 lightDir = normalize(pc.lightPos - fragWorldPos);

    // Calculate distance for attenuation
    float distance = length(pc.lightPos - fragWorldPos);
    float attenuation = 1.0 / (1.0 + 0.09 * distance + 0.032 * distance * distance);

    // Diffuse lighting
    float diff = max(dot(normal, lightDir), 0.0);
    vec3 diffuse = diff * pc.lightColor * pc.lightIntensity * attenuation;

    // Ambient lighting
    vec3 ambient = 0.2 * fragColor;

    // Combine lighting with object color
    vec3 result = (ambient + diffuse) * fragColor;

    outColor = vec4(result, 1.0);
}