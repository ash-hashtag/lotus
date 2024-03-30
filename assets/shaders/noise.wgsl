
// MIT License. © Stefan Gustavson, Munrocket
//
fn permute4(x: vec4f) -> vec4f { return ((x * 34. + 1.) * x) % vec4f(289.); }
fn fade2(t: vec2f) -> vec2f { return t * t * t * (t * (t * 6. - 15.) + 10.); }

fn perlinNoise2(P: vec2f) -> f32 {
    var Pi: vec4f = floor(P.xyxy) + vec4f(0., 0., 1., 1.);
    let Pf = fract(P.xyxy) - vec4f(0., 0., 1., 1.);
    Pi = Pi % vec4f(289.); // To avoid truncation effects in permutation
    let ix = Pi.xzxz;
    let iy = Pi.yyww;
    let fx = Pf.xzxz;
    let fy = Pf.yyww;
    let i = permute4(permute4(ix) + iy);
    var gx: vec4f = 2. * fract(i * 0.0243902439) - 1.; // 1/41 = 0.024...
    let gy = abs(gx) - 0.5;
    let tx = floor(gx + 0.5);
    gx = gx - tx;
    var g00: vec2f = vec2f(gx.x, gy.x);
    var g10: vec2f = vec2f(gx.y, gy.y);
    var g01: vec2f = vec2f(gx.z, gy.z);
    var g11: vec2f = vec2f(gx.w, gy.w);
    let norm = 1.79284291400159 - 0.85373472095314 * vec4f(dot(g00, g00), dot(g01, g01), dot(g10, g10), dot(g11, g11));
    g00 = g00 * norm.x;
    g01 = g01 * norm.y;
    g10 = g10 * norm.z;
    g11 = g11 * norm.w;
    let n00 = dot(g00, vec2f(fx.x, fy.x));
    let n10 = dot(g10, vec2f(fx.y, fy.y));
    let n01 = dot(g01, vec2f(fx.z, fy.z));
    let n11 = dot(g11, vec2f(fx.w, fy.w));
    let fade_xy = fade2(Pf.xy);
    let n_x = mix(vec2f(n00, n01), vec2f(n10, n11), vec2f(fade_xy.x));
    let n_xy = mix(n_x.x, n_x.y, fade_xy.y);
    return 2.3 * n_xy;
}


fn setABGR(color: vec4<u32>) -> u32 {
    return (color.r << 24) | (color.g << 16) | (color.b << 8) | color.a;
}

// Example: Get RGBA components of the color
fn getRGBA(color: u32) -> vec4<u32> {
    let r = (color >> 24) & 0xFFu;
    let g = (color >> 16) & 0xFFu;
    let b = (color >> 8) & 0xFFu;
    let a = color & 0xFFu;
    return vec4<u32>(r, g, b, a);
}


fn mul_and_clamp_to_255(val: f32) -> u32 {
    return clamp(u32(val * 255.0), u32(0), u32(255));
}

fn to_color32(color: vec4<f32>) -> u32 {
    let r = mul_and_clamp_to_255(color.w);
    let g = mul_and_clamp_to_255(color.z);
    let b = mul_and_clamp_to_255(color.y);
    let a = mul_and_clamp_to_255(color.x);
    
    // Pack components into a single 32-bit integer
    let result = (u32(r) << 24u) | (u32(g) << 16u) | (u32(b) << 8u) | u32(a);

    return result;
}



struct NoiseInput {
    seed: f32,
    offset: vec2<f32>,
    size: vec2<u32>,
}
@group(0) @binding(0)
var<uniform> noise_input: NoiseInput;

@group(0) @binding(1)
var<storage, read_write> outputBuffer: array<u32>; 


@compute @workgroup_size(1)
fn cm_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let frequency = 5.0;
    let colorMultiplier = 5.5;
    let colorAdditive = 0.5;
    let pixelCoord = global_id.xy;
    let offset = noise_input.offset + vec2<f32>(0.5, 0.0);
    var normalizedCoord = vec2f(pixelCoord) / vec2f(noise_input.size);
    var noise = perlinNoise2(normalizedCoord * frequency + offset.yx);
    var color = vec4<f32>(vec3<f32>(noise + colorAdditive), 1.0);

    let index = pixelCoord.x * noise_input.size.x + (noise_input.size.y - pixelCoord.y);
    outputBuffer[index] = to_color32(color);
}
