#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

struct SkyUniform {
    zenith_color: vec3<f32>,
    horizon_color: vec3<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<uniform> sky: SkyUniform;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let view_dir = normalize(in.world_position.xyz - view.world_position);

    // Symmetric gradient: horizon -> zenith the same way whether looking up or
    // down, so there's no distinct "ground" tint bleeding into view as the
    // camera gains altitude (the real ground platform is the only ground).
    let t = smoothstep(0.0, 0.4, abs(view_dir.y));
    let color = mix(sky.horizon_color, sky.zenith_color, t);

    return vec4<f32>(color, 1.0);
}
