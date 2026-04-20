#version 460

layout(location = 0) in vec3 f_normal;

layout(location = 0) out vec4 f_color;

void main() {
    f_color = vec4((f_normal + 1) / 2, 1.0);
}
