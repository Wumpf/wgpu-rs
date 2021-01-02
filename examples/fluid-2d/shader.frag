#version 450
#extension GL_EXT_samplerless_texture_functions : require

layout(location = 0) out vec4 out_Color;
layout(set = 0, binding = 0) uniform texture2D VelocityField;

void main() {
  out_Color = texelFetch(VelocityField, ivec2(gl_FragCoord.xy), 0);
}