// Portions adapted from Microsoft MiniEngine (c) Microsoft, MIT License
//
// Origin: the public repository `microsoft/DirectX-Graphics-Samples`, MiniEngine, files
// `Core/SSAO.cpp` and `Core/Shaders/Ao*.hlsl(i)`, author James Stanard. A copy of those files and
// of their MIT licence is kept under `third_party/MiniEngine-SSAO` (see `third_party/README.md`);
// the licence is carried with the distribution in `THIRD-PARTY-NOTICES.md`. MIT is compatible with
// the GPL-3.0 this reimplementation is distributed under.
//
// One level of the depth pyramid of AMD-055 REQ-SCN-083 *Mode MSVO*: the top-left texel of each
// two by two block of the level above, which is what `AoPrepareDepthBuffers2CS.hlsl` stores in its
// `DS2x` buffers. A *point* decimation and not an average on purpose — the original does the same,
// and averaging two depths across a silhouette would invent a surface that is in neither.
//
// `texelFetch` reads absolute texels, so this pass needs none of the scale bookkeeping the others
// carry: level `L` texel `(i, j)` is level `L − 1` texel `(2i, 2j)` whatever part of the
// attachment the frame drew into.

out vec4 oDepth;

uniform sampler2D uSource;

void main()
{
    oDepth = vec4(texelFetch(uSource, ivec2(gl_FragCoord.xy) * 2, 0).r, 0.0, 0.0, 1.0);
}
