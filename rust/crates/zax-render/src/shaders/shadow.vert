layout(location = 0) in vec3 aPosition;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec2 aUv;

uniform mat4 uModel;
uniform mat4 uLightViewProjection;

void main()
{
    vec3 local = aPosition;
    if (uDeform != 0)
    {
        local += zaxDeformation(uDeformTile, aPosition);
    }

    vec4 scene = uModel * vec4(local, 1.0);
    gl_Position = uLightViewProjection * vec4(scene.x, scene.y, -scene.z, 1.0);
}
