// The procedural Gradient and Fantasy skies of the wgpu backend (sky.vert, sky_noise.glsl,
// sky_procedural.glsl, sky_bodies.glsl, sky_gradient.frag, sky_fantasy.frag): REQ-SCN-050,
// REQ-SCN-051 and AMD-024 REQ-PUB-003. The renderer prepends common.wgsl and the constant
// ZAX_SRGB_DOUBLE.

struct Sky {
    right: vec4<f32>,
    up: vec4<f32>,
    forward: vec4<f32>,
    // xy: tan(fov / 2) times the aspect, and tan(fov / 2).
    tan_half: vec4<f32>,
    // Gradient: xyz the background colour M, w 1 when the stars show.
    background: vec4<f32>,
    // x: the twinkle angle.
    star: vec4<f32>,
    f0: vec4<f32>,
    f1: vec4<f32>,
    f2: vec4<f32>,
    f3: vec4<f32>,
    // x: the grey level of F0.
    gamma: vec4<f32>,
    sun_direction: vec4<f32>,
    // xyz linear colour, w gain.
    sun: vec4<f32>,
    sun_cos: vec4<f32>,
    moon_direction: vec4<f32>,
    moon: vec4<f32>,
    moon_cos: vec4<f32>,
    cloud: vec4<f32>,
    // x coverage, y scale.
    cloud_shape: vec4<f32>,
}

@group(0) @binding(0) var<uniform> sky: Sky;

struct SkyOut {
    @builtin(position) position: vec4<f32>,
    @location(0) direction: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> SkyOut {
    let ndc = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    var out: SkyOut;
    out.direction = sky.forward.xyz + sky.right.xyz * (ndc.x * sky.tan_half.x)
        + sky.up.xyz * (ndc.y * sky.tan_half.y);
    out.position = vec4<f32>(ndc, 1.0, 1.0);
    return out;
}

// ----------------------------------------------------------------- noise (sky_noise.glsl)

fn zax_sky_hash(p_in: vec3<f32>) -> f32 {
    var p = fract(p_in * vec3<f32>(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

fn zax_sky_noise(x: vec3<f32>) -> f32 {
    let i = floor(x);
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);

    let n000 = zax_sky_hash(i + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = zax_sky_hash(i + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = zax_sky_hash(i + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = zax_sky_hash(i + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = zax_sky_hash(i + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = zax_sky_hash(i + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = zax_sky_hash(i + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = zax_sky_hash(i + vec3<f32>(1.0, 1.0, 1.0));

    return mix(
        mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
        mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y),
        u.z);
}

fn zax_sky_fbm(x: vec3<f32>) -> f32 {
    var sum = 0.0;
    var weight = 0.0;
    var amplitude = 0.5;
    var at = x;
    for (var octave = 0; octave < 5; octave++) {
        sum += amplitude * zax_sky_noise(at);
        weight += amplitude;
        amplitude *= 0.5;
        at *= 2.03;
    }
    return sum / weight;
}

fn zax_sky_star_layer(d: vec3<f32>, scale: f32, density: f32, radius: f32, seed: f32) -> f32 {
    let p = d * scale + vec3<f32>(seed);
    let cell = floor(p);
    let f = p - cell;

    if (zax_sky_hash(cell + vec3<f32>(seed + 1.7)) > density) {
        return 0.0;
    }

    let centre = vec3<f32>(
        zax_sky_hash(cell + vec3<f32>(seed + 11.3)),
        zax_sky_hash(cell + vec3<f32>(seed + 23.9)),
        zax_sky_hash(cell + vec3<f32>(seed + 37.1)));
    let at = length(f - centre);

    let draw_value = zax_sky_hash(cell + vec3<f32>(seed + 53.7));
    let brightness = 0.40 + 0.60 * draw_value * draw_value;

    let size = radius * (0.5 + 0.5 * zax_sky_hash(cell + vec3<f32>(seed + 71.3)));
    return brightness * (1.0 - smoothstep(0.25 * size, size, at));
}

fn zax_sky_band(d: vec3<f32>, axis: vec3<f32>, width: f32) -> f32 {
    let t = dot(normalize(d), normalize(axis));
    return exp(-(t * t) / max(width * width, 1e-4));
}

// ----------------------------------------------------------------- fields (sky_procedural.glsl)

const ZAX_SKY_AXIS: vec3<f32> = vec3<f32>(0.37, 0.88, -0.30);

fn zax_sky_stars_r(d: vec3<f32>) -> f32 {
    let band = 0.55 + 0.75 * zax_sky_band(d, ZAX_SKY_AXIS, 0.42);
    return clamp(
        zax_sky_star_layer(d, 42.0, 0.030 * band, 0.13, 0.0)
            + zax_sky_star_layer(d, 78.0, 0.085 * band, 0.20, 7.0)
            + zax_sky_star_layer(d, 131.0, 0.110 * band, 0.32, 19.0)
            + zax_sky_star_layer(d, 197.0, 0.130 * band, 0.46, 43.0),
        0.0,
        1.0);
}

fn zax_sky_stars_b(d: vec3<f32>) -> f32 {
    return 0.35 + 0.65 * zax_sky_fbm(d * 9.0 + vec3<f32>(31.0, 17.0, 5.0));
}

fn zax_sky_starfield(d: vec3<f32>) -> f32 {
    let band = 0.6 + 0.8 * zax_sky_band(d, ZAX_SKY_AXIS, 0.45);
    return clamp(
        zax_sky_star_layer(d, 36.0, 0.022 * band, 0.09, 3.0)
            + zax_sky_star_layer(d, 71.0, 0.070 * band, 0.15, 19.0)
            + zax_sky_star_layer(d, 128.0, 0.105 * band, 0.25, 61.0)
            + zax_sky_star_layer(d, 211.0, 0.120 * band, 0.38, 97.0),
        0.0,
        1.0);
}

fn zax_sky_density_r(d: vec3<f32>) -> f32 {
    let slow = zax_sky_fbm(d * 1.35 + vec3<f32>(4.0, 1.0, 9.0));
    let ridge = 1.0 - abs(2.0 * zax_sky_fbm(d * 3.1 + vec3<f32>(13.0, 7.0, 2.0)) - 1.0);
    let fine = 1.0 - abs(2.0 * zax_sky_fbm(d * 6.7 + vec3<f32>(27.0, 5.0, 31.0)) - 1.0);
    let mass = slow * 0.55 + ridge * 0.30 + fine * 0.15;
    return clamp((mass - 0.57) * 3.00 + 0.46, 0.0, 1.0);
}

fn zax_sky_diffusion(d: vec3<f32>) -> vec3<f32> {
    let r = zax_sky_fbm(d * 3.4 + vec3<f32>(21.0, 3.0, 11.0));
    let g = zax_sky_fbm(d * 1.55 + vec3<f32>(2.0, 29.0, 6.0));
    let b = zax_sky_fbm(d * 2.2 + vec3<f32>(8.0, 14.0, 27.0));
    return clamp(
        vec3<f32>(
            r * 1.15 - 0.08,
            smoothstep(0.49, 0.85, g),
            smoothstep(0.44, 0.92, b)),
        vec3<f32>(0.0),
        vec3<f32>(1.0));
}

fn zax_sky_hue(d: vec3<f32>) -> vec3<f32> {
    let a = zax_sky_fbm(d * 1.05 + vec3<f32>(11.0, 2.0, 7.0));
    let b = zax_sky_fbm(d * 0.74 + vec3<f32>(5.0, 19.0, 23.0));
    let hue = mix(vec3<f32>(0.95, 0.16, 0.58), vec3<f32>(0.45, 0.20, 0.95), smoothstep(0.34, 0.70, a));
    return mix(hue, vec3<f32>(0.10, 0.80, 0.74), smoothstep(0.50, 0.86, b));
}

fn zax_sky_nebula_color(d: vec3<f32>, c: vec3<f32>) -> vec3<f32> {
    let level = max(max(c.r, c.g), c.b);
    return mix(c, zax_sky_hue(d) * level, 0.70);
}

// ----------------------------------------------------------------- bodies (sky_bodies.glsl)

fn zax_sky_disc(d: vec3<f32>, direction: vec3<f32>, cosines: vec2<f32>) -> f32 {
    return smoothstep(cosines.y, cosines.x, dot(d, direction));
}

fn zax_sky_bodies(d: vec3<f32>, color_in: vec3<f32>) -> vec3<f32> {
    var color = color_in;
    let cloud_gain = sky.cloud.w;
    if (cloud_gain > 0.0) {
        let band = 1.0 - smoothstep(0.08, 0.72, abs(d.y));
        let scale = max(sky.cloud_shape.y, 0.05);

        let coarse = zax_sky_fbm(d * scale + vec3<f32>(57.0, 13.0, 41.0));
        let fine = zax_sky_fbm(d * scale * 2.7 + vec3<f32>(9.0, 83.0, 17.0));
        let field = coarse * 0.72 + fine * 0.28;
        let threshold = 1.0 - sky.cloud_shape.x;
        var mass = smoothstep(threshold - 0.06, min(threshold + 0.40, 1.0), field);

        mass = mass * mass * (3.0 - 2.0 * mass);
        color = mix(color, sky.cloud.xyz, clamp(mass * band * cloud_gain, 0.0, 1.0));
    }

    let moon_gain = sky.moon.w;
    if (moon_gain > 0.0) {
        let disc = zax_sky_disc(d, sky.moon_direction.xyz, sky.moon_cos.xy);
        let craters = 0.78 + 0.22 * zax_sky_fbm(d * 180.0 + vec3<f32>(3.0, 71.0, 23.0));
        color = mix(color, sky.moon.xyz * craters, clamp(disc * moon_gain, 0.0, 1.0));
    }

    let sun_gain = sky.sun.w;
    if (sun_gain > 0.0) {
        let glow = zax_sky_disc(
            d, sky.sun_direction.xyz, vec2<f32>(sky.sun_cos.x, mix(sky.sun_cos.y, -1.0, 0.04)));
        color = mix(color, sky.sun.xyz, clamp(0.22 * glow * glow * sun_gain, 0.0, 1.0));

        let disc = zax_sky_disc(d, sky.sun_direction.xyz, sky.sun_cos.xy);
        color = mix(color, sky.sun.xyz, clamp(disc * sun_gain, 0.0, 1.0));
    }

    return color;
}

// ----------------------------------------------------------------- Gradient (sky_gradient.frag)

@fragment
fn fs_gradient(in: SkyOut) -> @location(0) vec4<f32> {
    let d = normalize(in.direction);
    let y = d.y;

    // REQ-SCN-050: Hc = 1.4 M and Bc = 0.6 M, never saturated before the conversion.
    let m = sky.background.xyz;
    let c = m + (1.4 * m - m) * pow(max(y, 0.0), 2.5);
    let g = c + (0.6 * m - c) * pow(max(-y, 0.0), 2.5);
    var lit = zax_to_linear3(g);

    if (sky.background.w > 0.5) {
        let ca = cos(sky.star.x);
        let sa = sin(sky.star.x);
        let turned = vec3<f32>(d.x * ca + d.z * sa, d.y, -d.x * sa + d.z * ca);
        let s = zax_sky_stars_r(d) * zax_sky_stars_b(turned);
        let k = clamp(s * min(abs(0.2 * y), 1.0), 0.0, 1.0);
        let e = zax_to_linear(0.5) * ZAX_SRGB_DOUBLE;
        lit = lit * (1.0 - k) + k * vec3<f32>(e);
    }

    lit = zax_sky_bodies(d, lit);
    return vec4<f32>(clamp(lit, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}

// ----------------------------------------------------------------- Fantasy (sky_fantasy.frag)

fn zax_smoothed(u_in: f32) -> f32 {
    let u = clamp(u_in, 0.0, 1.0);
    return u * u * (3.0 - 2.0 * u);
}

fn zax_density(x: vec3<f32>) -> f32 {
    return (zax_sky_density_r(normalize(x)) - 0.4) / 0.4;
}

@fragment
fn fs_fantasy(in: SkyOut) -> @location(0) vec4<f32> {
    let d = normalize(in.direction);
    let delta = zax_sky_diffusion(d);
    let u0 = zax_density(d);
    let gamma = sky.gamma.x;

    var color = zax_to_linear3(sky.f0.xyz);
    let alpha = select(1.0 - gamma, 0.0, gamma > 0.4);
    let field = zax_sky_starfield(d);
    let weight = clamp(clamp(field, 0.0, 1.0) * alpha, 0.0, 1.0);
    color = mix(color, zax_to_linear3(sky.f1.xyz * 0.5) * ZAX_SRGB_DOUBLE, weight);

    let halo = zax_to_linear3(zax_sky_nebula_color(d, sky.f0.xyz)) * ZAX_SRGB_DOUBLE * delta.g;
    color = mix(color, halo, clamp(2.0 * delta.g * zax_smoothed(1.0 - u0), 0.0, 1.0));

    let mean = (delta.g + delta.b) * 0.5;
    let base = zax_to_linear3(zax_sky_nebula_color(d, (sky.f2.xyz + sky.f3.xyz) * 0.5)) * ZAX_SRGB_DOUBLE * mean;
    color = mix(
        color,
        base,
        clamp(2.0 * mean * (1.0 - gamma) * 0.25 * zax_smoothed(u0), 0.0, 1.0));

    let d1 = d + 0.75 * vec3<f32>(0.2 * delta.b, -0.1 * delta.b, 1.0 * delta.b);
    let rho1 = zax_sky_diffusion(normalize(d1)).r;
    color = mix(
        color,
        zax_to_linear3(zax_sky_nebula_color(d1, sky.f2.xyz)) * ZAX_SRGB_DOUBLE * rho1,
        clamp(rho1 * 2.0 * (5.0 / 255.0) * zax_smoothed(zax_density(d1)), 0.0, 1.0));

    let d2 = d + vec3<f32>(1.0, 0.1, 0.2) * delta.g;
    let rho2 = zax_sky_diffusion(normalize(d2)).r;
    color = mix(
        color,
        zax_to_linear3(zax_sky_nebula_color(d2, sky.f3.xyz)) * ZAX_SRGB_DOUBLE * rho2,
        clamp(rho2 * 2.0 * (5.0 / 255.0) * zax_smoothed(zax_density(d2)), 0.0, 1.0));

    color = zax_sky_bodies(d, color);
    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
