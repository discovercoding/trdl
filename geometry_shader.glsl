#version 400

bool is_edge(vec3 bary1, vec3 bary2, vec3 edges) {
       return (edges.x > 0 && bary1.x <= 0.1 && bary2.x <= 0.1) ||
              (edges.y > 0 && bary1.y <= 0.1 && bary2.y <= 0.1) ||
              (edges.z > 0 && bary1.z <= 0.1 && bary2.z <= 0.1);
}

in vec3 te_edge[];
in vec3 te_bary[];
in vec3 te_color[];

out vec3 g_color;

layout(triangles) in;
layout(triangle_strip, max_vertices = 12) out;

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

    if (is_edge(te_bary[0], te_bary[1], te_edge[0])) {
        vec2 p0 = v0.xy;
        vec2 p1 = v1.xy;
        vec2 V = normalize(p1 - p0);
        vec2 N = vec2(-V.y, V.x) * 0.01;
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p0 - N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p0 + N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p1 - N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p1 + N, depth, 1); 
        EmitVertex();
        EndPrimitive();
    }
    if (is_edge(te_bary[1], te_bary[2], te_edge[1])) {
        vec2 p0 = v1.xy;
        vec2 p1 = v2.xy;
        vec2 V = normalize(p0 - p1);
        vec2 N = vec2(-V.y, V.x) * 0.01;
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p0 - N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p0 + N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p1 - N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p1 + N, depth, 1); 
        EmitVertex();
        EndPrimitive();
    }
    if (is_edge(te_bary[2], te_bary[0], te_edge[2])) {
        vec2 p0 = v2.xy;
        vec2 p1 = v0.xy;
        vec2 V = normalize(p0 - p1);
        vec2 N = vec2(-V.y, V.x) * 0.01;
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p0 - N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p0 + N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p1 - N, depth, 1); 
        EmitVertex();
        g_color = vec3(1.0, 0, 0);
        gl_Position = vec4(p1 + N, depth, 1); 
        EmitVertex();
        EndPrimitive();
    } 

}

