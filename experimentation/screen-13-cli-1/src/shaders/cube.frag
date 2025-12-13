#version 460 core

layout(location = 0) in vec3 color;

layout(location = 0) out vec4 vk_Color;

void main() {
    vk_Color = vec4(color, 1);
}