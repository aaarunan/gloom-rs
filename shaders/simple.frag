#version 430 core

layout(location=0) in vec4 fragColor;
out layout(location=0) vec4 color;

void main()
{
    color = fragColor;
}