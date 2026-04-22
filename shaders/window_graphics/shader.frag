#version 460

layout(location = 0) in vec3 f_normal;
layout(location = 1) in vec3 f_position;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 1) uniform FragmentData {
     vec3 light_pos;
     vec3 eye_pos;
} uniforms;

void main() {
     // f_color = vec4((f_normal + 1) / 2, 1.0);

     vec3 ambient = vec3(13) / 255;
     vec3 diffuse = vec3(204) / 255;
     vec3 specular = vec3(255) / 255;

     vec3 light_dir = normalize(uniforms.light_pos - f_position);
     vec3 eye_dir = normalize(uniforms.eye_pos - f_position);

     float diffuse_coef = max(dot(f_normal, light_dir), 0);
     float specular_coef = 0;
     if (diffuse_coef > 0) {
          vec3 refl_light_dir = reflect(-light_dir, f_normal);
          specular_coef = max(dot(eye_dir, refl_light_dir), 0);
          specular_coef = pow(specular_coef, 50);
     }

     f_color = vec4(ambient + diffuse_coef * diffuse + specular_coef * specular, 1.0);
}
