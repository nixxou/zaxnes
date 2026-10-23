uniform int uDeform;
uniform vec3 uDeformAmplitude;
uniform vec3 uDeformSize;
uniform vec3 uDeformTile;
uniform vec4 uDeformSpeed[3];
uniform vec4 uDeformPhase[3];
uniform float uTime;

float zaxDeformRow(float amplitude, float size, vec3 n, vec4 speed, vec4 phase)
{
    return amplitude * size
         * sin(n.x * speed.x + phase.x)
         * sin(n.y * speed.y + phase.y)
         * sin(n.z * speed.z + phase.z)
         * sin(uTime * speed.w + phase.w);
}

vec3 zaxDeformation(vec3 v)
{
    vec3 p = uDeformTile + v;
    vec3 n = vec3(
        uDeformSize.x == 0.0 ? 0.0 : p.x / uDeformSize.x,
        uDeformSize.y == 0.0 ? 0.0 : p.y / uDeformSize.y,
        uDeformSize.z == 0.0 ? 0.0 : p.z / uDeformSize.z);
    return vec3(
        zaxDeformRow(uDeformAmplitude.x, uDeformSize.x, n, uDeformSpeed[0], uDeformPhase[0]),
        zaxDeformRow(uDeformAmplitude.y, uDeformSize.y, n, uDeformSpeed[1], uDeformPhase[1]),
        zaxDeformRow(uDeformAmplitude.z, uDeformSize.z, n, uDeformSpeed[2], uDeformPhase[2]));
}
