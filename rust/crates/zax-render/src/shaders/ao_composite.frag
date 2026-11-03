// The composite of AMD-055 REQ-SCN-083: "elle multiplie la couleur de chaque pixel de la scène
// par un facteur `1 − O`, en espace linéaire, avant l'encodage sRGB de REQ-SCN-028".
//
// **No colour target and no copy of the scene.** A pass cannot sample the attachment it draws
// into, and it does not have to: with `glBlendFunc(GL_ZERO, GL_SRC_COLOR)` the destination becomes
// `dst × src`, so one full-screen triangle that outputs `1 − O` multiplies the linear buffer in
// place. The alpha is multiplied by 1, which leaves it alone — it is never a colour
// (REQ-SCN-035).
//
// Where nothing was drawn the occlusion is 0 and the factor exactly 1, so the skybox and the
// cleared background come through bit for bit, which is the invariant of the A-h variant.

in vec2 vUv;
out vec4 oFactor;

uniform sampler2D uOcclusion;
// The part of the occlusion target the frame drew into: the pass runs at half of each dimension
// of the render target except at `ao.quality` High, and the target follows the attachments'
// reserved room, so one scale serves both.
uniform vec2 uDrawnScale;

void main()
{
    float occlusion = texture(uOcclusion, vUv * uDrawnScale).r;
    oFactor = vec4(vec3(1.0 - occlusion), 1.0);
}
