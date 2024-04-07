
// MIT License. © Stefan Gustavson, Munrocket
//
fn permute4(x: vec4f) -> vec4f { return ((x * 34. + 1.) * x) % vec4f(289.); }
fn permute(x: vec4f) -> vec4f { return ((x * 34. + 1.) * x) % vec4f(289.); }
fn fade2(t: vec2f) -> vec2f { return t * t * t * (t * (t * 6. - 15.) + 10.); }


fn fade(t: vec3f) -> vec3f {return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);}


//	Classic Perlin 3D Noise
//	by Stefan Gustavson
//

fn snoise(P: vec3f) -> f32 {
    var Pi0 = floor(P); // Integer part for indexing
    var Pi1 = Pi0 + vec3f(1.0); // Integer part + 1
    // Pi0 = mod(Pi0, 289.0);
    // Pi1 = mod(Pi1, 289.0);
    Pi0 = Pi0 % vec3f(289.0);
    Pi1 = Pi1 % vec3f(289.0);
    var Pf0 = fract(P); // Fractional part for interpolation
    var Pf1 = Pf0 - vec3f(1.0); // Fractional part - 1.0
    var ix = vec4f(Pi0.x, Pi1.x, Pi0.x, Pi1.x);
    var iy = vec4f(Pi0.yy, Pi1.yy);
    var iz0 = Pi0.zzzz;
    var iz1 = Pi1.zzzz;

    var ixy = permute(permute(ix) + iy);
    var ixy0 = permute(ixy + iz0);
    var ixy1 = permute(ixy + iz1);

    var gx0 = ixy0 / 7.0;
    var gy0 = fract(floor(gx0) / 7.0) - 0.5;
    gx0 = fract(gx0);
    var gz0 = vec4f(0.5) - abs(gx0) - abs(gy0);
    var sz0 = step(gz0, vec4(0.0));
    gx0 -= sz0 * (step(vec4f(0.0), gx0) - 0.5);
    gy0 -= sz0 * (step(vec4f(0.0), gy0) - 0.5);

    var gx1 = ixy1 / 7.0;
    var gy1 = fract(floor(gx1) / 7.0) - 0.5;
    gx1 = fract(gx1);
    var gz1 = vec4f(0.5) - abs(gx1) - abs(gy1);
    var sz1 = step(gz1, vec4f(0.0));
    gx1 -= sz1 * (step(vec4f(0.0), gx1) - 0.5);
    gy1 -= sz1 * (step(vec4f(0.0), gy1) - 0.5);

    var g000 = vec3f(gx0.x,gy0.x,gz0.x);
    var g100 = vec3f(gx0.y,gy0.y,gz0.y);
    var g010 = vec3f(gx0.z,gy0.z,gz0.z);
    var g110 = vec3f(gx0.w,gy0.w,gz0.w);
    var g001 = vec3f(gx1.x,gy1.x,gz1.x);
    var g101 = vec3f(gx1.y,gy1.y,gz1.y);
    var g011 = vec3f(gx1.z,gy1.z,gz1.z);
    var g111 = vec3f(gx1.w,gy1.w,gz1.w);

    var norm0 = taylorInvSqrt(vec4f(dot(g000, g000), dot(g010, g010), dot(g100, g100), dot(g110, g110)));
    g000 *= norm0.x;
    g010 *= norm0.y;
    g100 *= norm0.z;
    g110 *= norm0.w;
    var norm1 = taylorInvSqrt(vec4f(dot(g001, g001), dot(g011, g011), dot(g101, g101), dot(g111, g111)));
    g001 *= norm1.x;
    g011 *= norm1.y;
    g101 *= norm1.z;
    g111 *= norm1.w;

    var n000 = dot(g000, Pf0);
    var n100 = dot(g100, vec3f(Pf1.x, Pf0.yz));
    var n010 = dot(g010, vec3f(Pf0.x, Pf1.y, Pf0.z));
    var n110 = dot(g110, vec3f(Pf1.xy, Pf0.z));
    var n001 = dot(g001, vec3f(Pf0.xy, Pf1.z));
    var n101 = dot(g101, vec3f(Pf1.x, Pf0.y, Pf1.z));
    var n011 = dot(g011, vec3f(Pf0.x, Pf1.yz));
    var n111 = dot(g111, Pf1);

    var fade_xyz = fade(Pf0);
    var n_z = mix(vec4f(n000, n100, n010, n110), vec4(n001, n101, n011, n111), fade_xyz.z);
    var n_yz = mix(n_z.xy, n_z.zw, fade_xyz.y);
    var n_xyz = mix(n_yz.x, n_yz.y, fade_xyz.x);
    return 2.2 * n_xyz;
} 


fn taylorInvSqrt(a: vec4f) -> vec4f {
    return 1.79284291400159 - 0.85373472095314 * a;
}

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


@compute @workgroup_size(64)
fn cm_main(
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    let frequency = 5.0;
    let colorMultiplier = 5.5;
    let colorAdditive = 0.5;
    let pixelCoord = vec2<u32>(global_id.x, global_id.y);
    let offset = noise_input.offset;
    var normalizedCoord = vec2f(pixelCoord) / vec2f(noise_input.size);

    var noise = snoise(vec3f(normalizedCoord * frequency + offset.yx, noise_input.seed * 10000.));
    var color = vec4f(vec3f(noise * colorMultiplier + colorAdditive), 1.0);

    let width = noise_input.size.x;
    let height = noise_input.size.y;

    let index = pixelCoord.x * width + (height - pixelCoord.y);
    // outputBuffer[index] = to_color32(color);
    outputBuffer[index] = to_color32(vec4f(vec2f(normalizedCoord.xy), 1.0, 1.0));
}
