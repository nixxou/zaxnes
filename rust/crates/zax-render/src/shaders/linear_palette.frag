out vec4 oColor;

// The 64 sRGB colours of the frame palette, one texel each.
uniform sampler2D uPalette;

void main()
{
    // One fragment per entry: the same transfer function the scene pass would apply to the
    // entry, on the same GPU, so the scene reads the very values it used to compute.
    vec3 srgb = texelFetch(uPalette, ivec2(int(gl_FragCoord.x), 0), 0).rgb;
    oColor = vec4(zaxToLinear3(srgb), 1.0);
}
