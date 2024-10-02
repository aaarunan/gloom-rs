#version 430 core

in vec3 position;
in vec4 color;
in vec3 normal;

out layout(location = 0) vec4 fragColor;
out layout(location = 1) vec3 fragNormal;

uniform layout(location = 0) mat4 t;

void main()
{
    gl_Position = t * vec4(position, 100.0);
    fragColor = color;
    fragNormal = normal;
}
