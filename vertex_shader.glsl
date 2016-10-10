#version 400

in vec3 in_position;
in vec2 in_control_1;
in vec2 in_control_2;
in uint in_edge;
in vec3 in_color;

out vec2 v_control_1;
out vec2 v_control_2;
out uint v_edge;
out vec3 v_color;

void main() {
    gl_Position = vec4(in_position, 1);
    v_control_1 = in_control_1;
    v_control_2 = in_control_2;
    v_edge = in_edge;
    v_color = in_color;
}