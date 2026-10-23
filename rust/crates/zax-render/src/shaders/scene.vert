layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec2 aUv;

uniform mat4 uModel;
uniform mat3 uNormalMatrix;
uniform float uNormalSign;
uniform mat4 uViewProjection;

out vec3 vScene;
out vec3 vNormal;
out vec2 vUv;

void main()
{
    vec3 local = aPosition;
    if (uDeform != 0)
    {
        local += zaxDeformation(aPosition);
    }

    vec4 scene = uModel * vec4(local, 1.0);
    vScene = scene.xyz;
    // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation.
    // uNormalSign is -1 for a mirrored placement, whose cofactor matrix points the
    // normal inwards (Q-069).
    vNormal = uNormalSign * (uNormalMatrix * aNormal);
    vUv = aUv;
    gl_Position = uViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
}
