in vec3 vDirection;
out vec4 oColor;

// M, the universal NES background colour of the current frame, in sRGB 0..1.
uniform vec3 uBackground;
uniform float uStars;
uniform float uStarAngle;

void main()
{
    vec3 d = normalize(vDirection);
    float y = d.y;

    // REQ-SCN-050: Hc = 1.4 M and Bc = 0.6 M, never saturated before the conversion.
    vec3 m = uBackground;
    vec3 c = m + (1.4 * m - m) * pow(max(y, 0.0), 2.5);
    vec3 g = c + (0.6 * m - c) * pow(max(-y, 0.0), 2.5);
    vec3 lit = zaxToLinear3(g);

    if (uStars > 0.5)
    {
        // The second sample is taken in d turned about Y, which is the rotation
        // R_Y(b) of REQ-SCN-007 in the left-handed scene frame.
        float ca = cos(uStarAngle);
        float sa = sin(uStarAngle);
        vec3 turned = vec3(d.x * ca + d.z * sa, d.y, -d.x * sa + d.z * ca);
        float s = zaxSkyStarsR(d) * zaxSkyStarsB(turned);
        float k = clamp(s * min(abs(0.2 * y), 1.0), 0.0, 1.0);
        // E, a slightly grey white: sRGB 0.5 doubled, about 0.98 in linear.
        float e = zaxToLinear(0.5) * ZAX_SRGB_DOUBLE;
        lit = lit * (1.0 - k) + k * vec3(e);
    }

    // REQ-PUB-003 (Q-064): the sun, the moon and the clouds, all off by default.
    lit = zaxSkyBodies(d, lit);

    // R2-009: linear out; the sRGB encode is the resolve pass.
    oColor = vec4(clamp(lit, 0.0, 1.0), 1.0);
}
