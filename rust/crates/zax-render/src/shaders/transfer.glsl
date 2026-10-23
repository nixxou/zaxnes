float zaxToLinear(float c)
{
    return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4);
}

float zaxToSrgb(float c)
{
    return c <= 0.0031308 ? c * 12.92 : 1.055 * pow(c, 1.0 / 2.4) - 0.055;
}

vec3 zaxToLinear3(vec3 c)
{
    return vec3(zaxToLinear(c.r), zaxToLinear(c.g), zaxToLinear(c.b));
}

vec3 zaxToSrgb3(vec3 c)
{
    return vec3(zaxToSrgb(c.r), zaxToSrgb(c.g), zaxToSrgb(c.b));
}
