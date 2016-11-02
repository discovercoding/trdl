#version 400

layout(vertices = 3) out;

in vec2 v_control_1[];
in vec2 v_control_2[];
in float v_edge[];
in vec3 v_color[];
in vec3 v_stroke_color[];

out vec2 tc_control_1[];
out vec2 tc_control_2[];
out float tc_edge[];
out vec3 tc_color[];
out vec3 tc_stroke_color[];

uniform int outer_tess;
uniform int inner_tess;

void main() {
    tc_control_1[gl_InvocationID] = v_control_1[gl_InvocationID];
    tc_control_2[gl_InvocationID] = v_control_2[gl_InvocationID];
    tc_edge[gl_InvocationID] = v_edge[gl_InvocationID];
    tc_color[gl_InvocationID] = v_color[gl_InvocationID];
    tc_stroke_color[gl_InvocationID] = v_stroke_color[gl_InvocationID];
    gl_out[gl_InvocationID].gl_Position = gl_in[gl_InvocationID].gl_Position;
    if (gl_InvocationID == 0) {
        gl_TessLevelInner[0] = inner_tess;
        gl_TessLevelOuter[0] = outer_tess;
        gl_TessLevelOuter[1] = outer_tess;
        gl_TessLevelOuter[2] = outer_tess;
    }
}