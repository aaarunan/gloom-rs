#version 430 core

in vec3 position;
in vec4 color;

out layout(location=0) vec4 fragColor;

void main()
{
    gl_Position = vec4(position, 1.0f);
    fragColor = color;
}