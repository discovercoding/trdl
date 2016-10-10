#version 400

layout (triangles, equal_spacing, ccw) in;
in vec2 tc_control_1[];
in vec2 tc_control_2[];
in uint tc_edge[];
in vec3 tc_color[];

out vec3 te_bary;
out vec3 te_edge;
out vec3 te_color;

void main() {

    float s = gl_TessCoord.x;
    float t = gl_TessCoord.y;
    float u = gl_TessCoord.z;

    float s_sq = s * s;
    float t_sq = t * t;
    float u_sq = u * u;

    vec2 a   = gl_in[0].gl_Position.xy;
    vec2 ab0 = tc_control_1[0].xy;
    vec2 ab1 = tc_control_2[0].xy;
    vec2 b   = gl_in[1].gl_Position.xy;
    vec2 bc0 = tc_control_1[1].xy;
    vec2 bc1 = tc_control_2[1].xy;
    vec2 c   = gl_in[2].gl_Position.xy;
    vec2 ca0 = tc_control_1[2].xy;
    vec2 ca1 = tc_control_2[2].xy;
    vec2 ce = a + ab0 + ab1 + b + bc0 + bc1 + c + ca0 + ca1;
    ce /= 9.0;

    vec2 pos = vec2(
              a*s*s_sq + 3*ab0*s_sq*t + 3*ab1*s*t_sq +
              b*t*t_sq + 3*bc0*t_sq*u + 3*bc1*t*u_sq +
              c*u*u_sq + 3*ca0*u_sq*s + 3*ca1*u*s_sq
              + 6*ce*s*t*u);

    gl_Position = vec4(pos, gl_in[0].gl_Position.z, 1.0);
    te_bary = vec3(s, t, u);
    te_edge = vec3(tc_edge[0], tc_edge[1], tc_edge[2]);
    te_color = tc_color[0];
}

