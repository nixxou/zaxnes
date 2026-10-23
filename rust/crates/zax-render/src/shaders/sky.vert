uniform vec3 uRight;
uniform vec3 uUp;
uniform vec3 uForward;
uniform vec2 uTanHalf;

out vec3 vDirection;

void main()
{
    vec2 ndc = vec2((gl_VertexID << 1) & 2, gl_VertexID & 2) * 2.0 - 1.0;
    vDirection = uForward + uRight * (ndc.x * uTanHalf.x) + uUp * (ndc.y * uTanHalf.y);
    gl_Position = vec4(ndc, 1.0, 1.0);
}
