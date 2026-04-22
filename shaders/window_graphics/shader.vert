#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;

layout(location = 0) out vec3 f_normal;
layout(location = 1) out vec3 f_position;

layout(set = 0, binding = 0) uniform VertexData {
    mat4 mvp;
} uniforms;

void main() {
    f_normal = normalize(normal);
    f_position = position;
    gl_Position = uniforms.mvp * vec4(position, 1.0);
}
