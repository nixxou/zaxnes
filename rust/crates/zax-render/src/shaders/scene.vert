layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec2 aUv;

// The placement of each instance of a batched draw: the model matrix, the columns of the
// normal (cofactor) matrix and the tile position of the deformation (xyz). The renderer fills
// one block range per draw; 128 instances of 128 bytes fill exactly the 16 KiB every
// implementation grants a uniform block.
struct ZaxInstance
{
    mat4 model;
    vec4 normal[3];
    vec4 deformTile;
};

layout(std140) uniform ZaxInstances
{
    ZaxInstance uInstances[128];
};

uniform float uNormalSign;
uniform mat4 uViewProjection;

out vec3 vScene;
out vec3 vNormal;
out vec2 vUv;

void main()
{
    ZaxInstance instance = uInstances[gl_InstanceID];
    vec3 local = aPosition;
    if (uDeform != 0)
    {
        local += zaxDeformation(instance.deformTile.xyz, aPosition);
    }

    vec4 scene = instance.model * vec4(local, 1.0);
    vScene = scene.xyz;
    // REQ-SCN-028: the normal is the mesh normal, unchanged by the deformation.
    // uNormalSign is -1 for a mirrored placement, whose cofactor matrix points the
    // normal inwards (Q-069).
    mat3 normalMatrix = mat3(instance.normal[0].xyz, instance.normal[1].xyz, instance.normal[2].xyz);
    vNormal = uNormalSign * (normalMatrix * aNormal);
    vUv = aUv;
    gl_Position = uViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
}
