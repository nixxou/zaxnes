in vec2 vUv;
out vec4 oColor;

uniform sampler2D uLinear;

void main()
{
    vec4 linear = texture(uLinear, vUv);

    // REQ-SCN-035: the alpha channel is never converted.
    oColor = vec4(zaxToSrgb3(clamp(linear.rgb, 0.0, 1.0)), linear.a);
}
