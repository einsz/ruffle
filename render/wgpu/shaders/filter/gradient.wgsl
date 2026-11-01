// NOTE: The `shader_filter_common.wgsl` source is prepended to this before compilation.

struct Filter {
    // The gradient, 256 straight-alpha colors.
    colors: array<vec4<f32>, 256>,
    // The area that the filter draws to, in pixels (x_min, y_min, x_max, y_max).
    // The gradient's colors also show where there's no source, but only within it.
    area: vec4<f32>,
    // Where the source starts in the blurred texture (xy), and the offset of
    // the filter (zw), in pixels.
    blur: vec4<f32>,
    strength: f32,
    filter_type: u32,      // 0 outer, 1 inner, 2 full
    knockout: u32,         // a wasteful bool, but we need to be aligned anyway
    bevel: u32,            // 0 gradient glow, 1 gradient bevel
}

@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> filter_args: Filter;
@group(0) @binding(3) var blurred: texture_2d<f32>;

@vertex
fn main_vertex(in: filter__VertexInput) -> filter__VertexOutput {
    return filter__main_vertex(in);
}

// The alpha of a texel of the blurred texture, or 0 outside it.
fn blurred_alpha(texel: vec2<i32>) -> f32 {
    if (any(texel < vec2<i32>(0)) || any(texel >= vec2<i32>(textureDimensions(blurred)))) {
        return 0.0;
    }
    return textureLoad(blurred, texel, 0).a;
}

// The blurred alpha at a position in texels (whole numbers at texel centers).
fn sample_blur(position: vec2<f32>) -> f32 {
    // The offset needn't be whole pixels, so the alpha is interpolated. Like
    // Flash, this also uses pixels outside the source rectangle, if any.
    let texel = vec2<i32>(floor(position));
    let weight = position - floor(position);
    let top = mix(blurred_alpha(texel), blurred_alpha(texel + vec2<i32>(1, 0)), weight.x);
    let bottom = mix(
        blurred_alpha(texel + vec2<i32>(0, 1)),
        blurred_alpha(texel + vec2<i32>(1, 1)),
        weight.x,
    );
    // Flash works with the alpha as an integer, rounded down.
    return floor(mix(top, bottom, weight.y) * 255.0 + 0.001);
}

@fragment
fn main_fragment(in: filter__VertexOutput) -> @location(0) vec4<f32> {
    let knockout = filter_args.knockout > 0u;
    let dest = textureSample(texture, texture_sampler, in.uv);
    // Computed from the pixel rather than interpolated coordinates, so that
    // whole offsets give exactly the alpha of a texel.
    let position = in.position.xy - 0.5 + filter_args.blur.xy;
    let blur_ahead = sample_blur(position + filter_args.blur.zw);
    let blur_behind = sample_blur(position - filter_args.blur.zw);

    // The blurred alpha picks a color from the gradient: for a glow, the
    // alpha behind the pixel; for a bevel, the difference between the alpha
    // ahead and behind, with no difference in the middle.
    var index: f32;
    if (filter_args.bevel > 0u) {
        index = floor((256.0 + (blur_ahead - blur_behind) * filter_args.strength) / 2.0);
    } else {
        index = floor(blur_behind * filter_args.strength);
    }
    let color = filter_args.colors[u32(clamp(index, 0.0, 255.0))];
    var glow = vec4<f32>(color.rgb * color.a, color.a);
    let area = filter_args.area;
    if (in.position.x < area.x || in.position.y < area.y || in.position.x > area.z || in.position.y > area.w) {
        glow = vec4<f32>(0.0);
    }

    if (filter_args.filter_type == 2u) {
        if (knockout) {
            return glow;
        } else {
            return glow + dest * (1.0 - glow.a);
        }
    } else if (filter_args.filter_type == 1u) {
        if (knockout) {
            return glow * dest.a;
        } else {
            return glow * dest.a + dest * (1.0 - glow.a);
        }
    } else {
        if (knockout) {
            return glow * (1.0 - dest.a);
        } else {
            return dest + glow * (1.0 - dest.a);
        }
    }
}
