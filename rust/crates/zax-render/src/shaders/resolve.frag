in vec2 vUv;
out vec4 oColor;

uniform sampler2D uLinear;
// The part of the texture drawn into (1, 1 for all of it): the attachments may be larger than
// the picture when the host reserved room.
uniform vec2 uUvScale;

void main()
{
    vec4 linear = texture(uLinear, vUv * uUvScale);

    // REQ-SCN-035: the alpha channel is never converted.
    oColor = vec4(zaxToSrgb3(clamp(linear.rgb, 0.0, 1.0)), linear.a);
}
