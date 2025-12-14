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

    // normal visualization
    outColor = vec4(result, 1.0);

    //
    // debugging techniques below
    //

    // use this to vizualize the Z buffer
    //float depth = gl_FragCoord.z;
    //outColor = vec4(depth, depth, depth, 1.0);

    // use this to visualize the Z is positive or negative
    //vec3 viewPos = vec3(pc.mvp * vec4(fragWorldPos, 1.0));
    //if (viewPos.z > 0.0) {
    //     outColor = vec4(0.0, 1.0, 0.0, 1.0); // Green for positive Z
    //} else {
    //    outColor = vec4(1.0, 0.0, 0.0, 1.0); // Red for negative Z
    //}

    // use this to visualize the depth contrast
    //float depth = gl_FragCoord.z;

    // Apply a contrast stretch to make small differences visible
    // If depth is always near 0.5, this will make it 0.0
    // If depth is always near 1.0, this will make it 1.0
    //float contrastDepth = (depth - 0.5) * 10.0 + 0.5; // Stretch around 0.5
    //contrastDepth = clamp(contrastDepth, 0.0, 1.0);

    //outColor = vec4(contrastDepth, contrastDepth, contrastDepth, 1.0);

    // Direct test: if depth is exactly 0, output red
    //float depth = gl_FragCoord.z;

    //if (depth == 0.0) {
    //    outColor = vec4(1.0, 0.0, 0.0, 1.0); // Red = depth is exactly 0
    //} else if (depth < 0.001) {
    //    outColor = vec4(1.0, 1.0, 0.0, 1.0); // Yellow = depth is very small but not 0
    //} else {
    //    outColor = vec4(0.0, 1.0, 0.0, 1.0); // Green = depth is reasonable
    //}
}