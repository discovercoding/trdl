#version 400

in vec3 in_position;
in vec2 in_control_1;
in vec2 in_control_2;
in float in_edge;
in vec3 in_color;
in vec3 in_stroke_color;

out vec2 v_control_1;
out vec2 v_control_2;
out float v_edge;
out vec3 v_color;
out vec3 v_stroke_color;

uniform mat4 projection;

void main() {
    gl_Position = projection * vec4(in_position, 1);
    v_control_1 = (projection * vec4(in_control_1, 0, 1)).xy;
    v_control_2 = (projection * vec4(in_control_2, 0, 1)).xy;
    v_edge = in_edge;
    v_color = in_color;
    v_stroke_color = in_stroke_color;
}