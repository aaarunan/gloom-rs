#version 430 core

layout(location = 0) in vec4 fragColor;
out layout(location = 0) vec4 color;

vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));
layout(location = 1) in vec3 fragNormal;

void main()
{
    float diffuse = max(0.0, dot(fragNormal, -lightDirection));
    color = vec4(fragColor.rgb * diffuse, fragColor.a);
}

