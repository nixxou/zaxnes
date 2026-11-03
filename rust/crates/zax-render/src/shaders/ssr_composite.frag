// The composite of AMD-055 REQ-SCN-084: "le mélange se fait en espace linéaire, avant l'encodage
// sRGB de REQ-SCN-028", with `(1 − i) · surface + i · reflet`.
//
// The trace holds its result premultiplied (`rgb = w · c`, `a = w`), so with
// `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` the destination becomes
// `w · c + (1 − w) · dst` — exactly the blend the requirement writes, in the linear buffer, in
// place, with no second colour target and no copy of the scene.
//
// **The mask is applied again here, at full resolution**, and that is not a belt: the trace runs
// at half of each dimension, so sampling it back up with a linear filter would spread the
// reflection half an output pixel past the floor's silhouette. Multiplying by the mask the frame
// itself rasterised makes "aucun pixel qui n'est pas une surface de sol n'est modifié" — the
// invariants of the variants R-m, R-s and R-h — true pixel by pixel rather than nearly.

in vec2 vUv;
out vec4 oReflection;

uniform sampler2D uReflection;
uniform sampler2D uMask;
// The part of the targets the frame drew into: both follow the attachments' reserved room, so
// one scale serves both.
uniform vec2 uDrawnScale;

void main()
{
    vec4 reflection = texture(uReflection, vUv * uDrawnScale);
    float mask = texture(uMask, vUv * uDrawnScale).r;
    oReflection = reflection * mask;
}
