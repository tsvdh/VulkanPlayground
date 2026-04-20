#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 f_normal;

layout(set = 0, binding = 0) uniform Data {
    mat4 mvp;
    vec3 light_dir;
} uniforms;

void main() {
    f_normal = normalize(normal);
    gl_Position = uniforms.mvp * vec4(position, 1.0);
}
