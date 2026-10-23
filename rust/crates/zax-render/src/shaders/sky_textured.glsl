uniform samplerCube uStarMap;
uniform samplerCube uStarfield;
uniform samplerCube uDensity;
uniform samplerCube uDiffusion;

// REQ-SCN-050: s is the red channel of Stars_LP_01 in d times its blue channel in the
// turned direction.
float zaxSkyStarsR(vec3 d) { return texture(uStarMap, d).r; }
float zaxSkyStarsB(vec3 d) { return texture(uStarMap, d).b; }

float zaxSkyStarfield(vec3 d) { return texture(uStarfield, d).r; }
float zaxSkyDensityR(vec3 d) { return texture(uDensity, d).r; }
vec3 zaxSkyDiffusion(vec3 d) { return texture(uDiffusion, d).rgb; }

// With real cubemaps the colours of REQ-SCN-051 are used exactly as the formula
// gives them; the hue field below belongs to the procedural sky alone.
vec3 zaxSkyNebulaColor(vec3 d, vec3 c) { return c; }
