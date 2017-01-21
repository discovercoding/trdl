#version 400

in vec3 g_color;
layout ( location = 0 ) out vec4 frag_color;

// Set the fragment color.
void main() {
    frag_color = vec4(g_color, 1.0);
}
