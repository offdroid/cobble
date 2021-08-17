// Taken from https://github.com/bevyengine/bevy/blob/cf221f9659127427c99d621b76c8085c4860e2ef/crates/bevy_pbr/src/render_graph/pbr_pipeline/pbr.vert
// Modified to support 2D array textures
#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec2 Vertex_Uv;
layout(location = 3) in uint Vertex_Layer;

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 4) in vec4 Vertex_Tangent;
#endif

layout(location = 0) out vec3 v_WorldPosition;
layout(location = 1) out vec3 v_WorldNormal;
layout(location = 2) out vec3 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

#ifdef STANDARDMATERIAL_NORMAL_MAP
layout(location = 3) out vec4 v_WorldTangent;
#endif

layout(set = 2, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    vec4 world_position = Model * vec4(Vertex_Position, 1.0);
    v_WorldPosition = world_position.xyz;
    v_WorldNormal = mat3(Model) * Vertex_Normal;
    v_Uv = vec3(Vertex_Uv, Vertex_Layer);
#ifdef STANDARDMATERIAL_NORMAL_MAP
    v_WorldTangent = vec4(mat3(Model) * Vertex_Tangent.xyz, Vertex_Tangent.w);
#endif
    gl_Position = ViewProj * world_position;
}
