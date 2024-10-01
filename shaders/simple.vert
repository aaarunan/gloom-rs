#version 430 core

in vec3 position;
in vec4 color;

out layout(location = 0) vec4 fragColor;
uniform layout(location = 0) mat4 t;

void main()
{
    gl_Position = t * vec4(position, 1);
    fragColor = color;
}
