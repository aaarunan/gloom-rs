#version 430 core

layout(location = 0) in vec4 fragColor;
out layout(location = 0) vec4 color;

vec3 lightDirection = normalize(vec3(0.8, -0.5, 0.6));
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec3 fragPosition;

uniform layout(location = 2) vec3 viewPosition;
float shininess = 32.0;
float ambient_scalar = 0.05;

void main()
{
    vec3 ambient = ambient_scalar * fragColor.rgb;

    vec3 norm = normalize(fragNormal);
    float diffuseStrength = max(0.0, dot(norm, -lightDirection));
    vec3 diffuse = diffuseStrength * fragColor.rgb;

    vec3 viewDir = normalize(viewPosition - fragPosition);
    vec3 reflectDir = reflect(lightDirection, norm);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), shininess);
    vec3 specular = spec * vec3(1.0);

    vec3 finalColor = ambient + diffuse + specular;
    color = vec4(finalColor, fragColor.a);
}
