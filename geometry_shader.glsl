#version 400

in vec3 te_edge[];
in vec3 te_bary[];
in vec3 te_color[];

in vec2 te_tan_ab[];
in vec2 te_tan_bc[];
in vec2 te_tan_ca[];

out vec3 g_color;

layout(triangles) in;
layout(triangle_strip, max_vertices = 12) out;

//bool is_edge(vec3 bary1, vec3 bary2, vec3 edges) {
//       return (edges.x > 0 && bary1.x <= 0.000000001 && bary2.x <= 0.000000001) ||
//              (edges.y > 0 && bary1.y <= 0.000000001 && bary2.y <= 0.000000001) ||
//              (edges.z > 0 && bary1.z <= 0.000000001 && bary2.z <= 0.000000001);
//}

int is_edge(vec3 bary1, vec3 bary2, vec3 edges) {
    if (edges.x > 0 && bary1.x <= 0.000000001 && bary2.x <= 0.000000001) { return 1; }
    else if (edges.y > 0 && bary1.y <= 0.000000001 && bary2.y <= 0.000000001) { return 2; }
    else if (edges.z > 0 && bary1.z <= 0.000000001 && bary2.z <= 0.000000001) { return 3; }
    else { return 0; }
}

//void make_edge(vec2 p0, vec2 p1, float depth) {
//    vec2 V = normalize(p1 - p0);
//    vec2 N = vec2(-V.y, V.x) * 0.01;
//    g_color = vec3(1.0, 0, 0);
//    gl_Position = vec4(p0 - N, depth, 1); 
//    EmitVertex();
//    g_color = vec3(1.0, 0, 0);
//    gl_Position = vec4(p0 + N, depth, 1); 
//    EmitVertex();
//    g_color = vec3(1.0, 0, 0);
//    gl_Position = vec4(p1 - N, depth, 1); 
//    EmitVertex();
//    g_color = vec3(1.0, 0, 0);
//    gl_Position = vec4(p1 + N, depth, 1); 
//    EmitVertex();
//    EndPrimitive();
//}

void make_edge(vec2 p0, vec2 p1, float depth, vec2 tan0, vec2 tan1) {
    tan0 = normalize(tan0);
    vec2 perp0 = vec2(-tan0.y, tan0.x) * 0.02;
    tan1 = normalize(tan1);
    vec2 perp1 = vec2(-tan1.y, tan1.x) * 0.02; 

    g_color = vec3(1.0, 0, 0);
    gl_Position = vec4(p0 - perp0, depth, 1); 
    EmitVertex();
    g_color = vec3(1.0, 0, 0);
    gl_Position = vec4(p0 + perp0, depth, 1); 
    EmitVertex();
    g_color = vec3(1.0, 0, 0);
    gl_Position = vec4(p1 - perp1, depth, 1); 
    EmitVertex();
    g_color = vec3(1.0, 0, 0);
    gl_Position = vec4(p1 + perp1, depth, 1); 
    EmitVertex();
    EndPrimitive();
}

void main() {
    vec3 v0 = gl_in[0].gl_Position.xyz;
    vec3 v1 = gl_in[1].gl_Position.xyz;
    vec3 v2 = gl_in[2].gl_Position.xyz;

    g_color = te_color[0];
    gl_Position = vec4(v0, 1);
    EmitVertex();

    g_color = te_color[1];
    gl_Position = vec4(v1, 1);
    EmitVertex();

    g_color = te_color[2];
    gl_Position = vec4(v2, 1);
    EmitVertex();

    EndPrimitive();

    float depth = v0.z + 0.000001;

    int edge = is_edge(te_bary[0], te_bary[1], te_edge[0]);
    if (edge == 1) {
        make_edge(v0.xy, v1.xy, depth, te_tan_bc[0], te_tan_bc[1]);
    } else if (edge == 2) {
        make_edge(v0.xy, v1.xy, depth, te_tan_ca[0], te_tan_ca[1]);
    } else if (edge == 3) {
        make_edge(v0.xy, v1.xy, depth, te_tan_ab[0], te_tan_ab[1]);
    }

    edge = is_edge(te_bary[1], te_bary[2], te_edge[1]);
    if (edge == 1) {
        make_edge(v1.xy, v2.xy, depth, te_tan_bc[1], te_tan_bc[2]);
    } else if (edge == 2) {
        make_edge(v1.xy, v2.xy, depth, te_tan_ca[1], te_tan_ca[2]);
    } else if (edge == 3) {
        make_edge(v1.xy, v2.xy, depth, te_tan_ab[1], te_tan_ab[2]);
    }

    edge = is_edge(te_bary[2], te_bary[0], te_edge[2]);
    if (edge == 1) {
        make_edge(v2.xy, v0.xy, depth, te_tan_bc[2], te_tan_bc[0]);
    } else if (edge == 2) {
        make_edge(v2.xy, v0.xy, depth, te_tan_ca[2], te_tan_ca[0]);
    } else if (edge == 3) {
        make_edge(v2.xy, v0.xy, depth, te_tan_ab[2], te_tan_ab[0]);
    }
}

