#version 400

in vec3 in_position;
in vec2 in_control_1;
in vec2 in_control_2;
in float in_edge;
in vec3 in_color;
in vec3 in_stroke_color;
in int in_do_fill;

out vec2 v_control_1;
out vec2 v_control_2;
out float v_edge;
out vec3 v_color;
out vec3 v_stroke_color;
out int v_do_fill;

uniform mat4 projection;

// Apply the projection matrix and pass on needed info.
void main() {
    gl_Position = projection * vec4(in_position, 1);
    v_control_1 = (projection * vec4(in_control_1, 0, 1)).xy;
    v_control_2 = (projection * vec4(in_control_2, 0, 1)).xy;
    v_edge = in_edge;
    v_color = in_color;
    v_stroke_color = in_stroke_color;
    v_do_fill = in_do_fill;
}