// A cheap, well-behaved 3D hash in [0, 1). No trigonometry, so no precision cliff when
// the lattice coordinate grows.
float zaxSkyHash(vec3 p)
{
    p = fract(p * vec3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}

// Trilinear value noise with a smoothstep fade, mean 0.5.
float zaxSkyNoise(vec3 x)
{
    vec3 i = floor(x);
    vec3 f = x - i;
    vec3 u = f * f * (3.0 - 2.0 * f);

    float n000 = zaxSkyHash(i + vec3(0.0, 0.0, 0.0));
    float n100 = zaxSkyHash(i + vec3(1.0, 0.0, 0.0));
    float n010 = zaxSkyHash(i + vec3(0.0, 1.0, 0.0));
    float n110 = zaxSkyHash(i + vec3(1.0, 1.0, 0.0));
    float n001 = zaxSkyHash(i + vec3(0.0, 0.0, 1.0));
    float n101 = zaxSkyHash(i + vec3(1.0, 0.0, 1.0));
    float n011 = zaxSkyHash(i + vec3(0.0, 1.0, 1.0));
    float n111 = zaxSkyHash(i + vec3(1.0, 1.0, 1.0));

    return mix(
        mix(mix(n000, n100, u.x), mix(n010, n110, u.x), u.y),
        mix(mix(n001, n101, u.x), mix(n011, n111, u.x), u.y),
        u.z);
}

// Five octaves of value noise, normalised back to [0, 1].
float zaxSkyFbm(vec3 x)
{
    float sum = 0.0;
    float weight = 0.0;
    float amplitude = 0.5;
    vec3 at = x;
    for (int octave = 0; octave < 5; octave++)
    {
        sum += amplitude * zaxSkyNoise(at);
        weight += amplitude;
        amplitude *= 0.5;
        at *= 2.03;
    }

    return sum / weight;
}

// One sparse layer of round stars: the direction is cut into cells, a fraction of them
// holds a star, and each star is a soft disc inside its cell. 'radius' is the disc's
// size in cells - a star has to stay a point, so a coarse layer takes a small
// fraction of its cell and a fine one a larger one.
float zaxSkyStarLayer(vec3 d, float scale, float density, float radius, float seed)
{
    vec3 p = d * scale + vec3(seed);
    vec3 cell = floor(p);
    vec3 f = p - cell;

    if (zaxSkyHash(cell + vec3(seed + 1.7)) > density)
    {
        return 0.0;
    }

    vec3 centre = vec3(
        zaxSkyHash(cell + vec3(seed + 11.3)),
        zaxSkyHash(cell + vec3(seed + 23.9)),
        zaxSkyHash(cell + vec3(seed + 37.1)));
    float at = length(f - centre);

    // The brightest stars are the rarest: the square of a uniform draw. The floor
    // matters - REQ-SCN-050 multiplies the field by at most 0.2, so a star that
    // starts dim never reaches the image at all.
    float draw = zaxSkyHash(cell + vec3(seed + 53.7));
    float brightness = 0.40 + 0.60 * draw * draw;

    // A star of this layer is between half and one radius across.
    float size = radius * (0.5 + 0.5 * zaxSkyHash(cell + vec3(seed + 71.3)));
    return brightness * (1.0 - smoothstep(0.25 * size, size, at));
}

// A band of denser stars across the sky, the way a galactic plane reads: 1 on the
// great circle normal to 'axis', fading to 0 a few tens of degrees away.
float zaxSkyBand(vec3 d, vec3 axis, float width)
{
    float t = dot(normalize(d), normalize(axis));
    return exp(-(t * t) / max(width * width, 1e-4));
}
