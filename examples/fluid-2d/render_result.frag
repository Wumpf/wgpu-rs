#version 450

layout(location = 0) out vec4 out_Color;
layout(set = 0, binding = 0, rg32f) readonly uniform image2D VelocityField;

void main() { out_Color = imageLoad(VelocityField, ivec2(gl_FragCoord.xy)); }