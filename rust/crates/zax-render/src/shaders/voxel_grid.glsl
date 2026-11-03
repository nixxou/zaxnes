// The voxel grid of AMD-055 REQ-SCN-081, compiled only into the ZAX_VOXEL_GRID variants.
//
// The grid modulates the **albedo** of a tile mesh's faces and of the backdrop box's faces,
// before any lighting: for a tile in the mesh's local frame of REQ-E3D-504 (so before the
// animated deformation, before the object's transform and before the curvature — the grooves
// therefore follow the object's scale, its rotation and the curved surface), and for the backdrop
// in scene units. `uVoxelCell` is the cell side `N = 2^spl.voxel_grid`, times the scene scale for
// the backdrop.

uniform float uVoxelCell;

// The albedo factor at the point `p` on a face of unit normal `n`, both in the frame the grid is
// evaluated in, for a cell side of `cell`.
float zaxVoxelGrid(vec3 p, vec3 n, float cell)
{
    vec3 an = abs(n);

    // Alignment on an axis: `m = max |n_a|` and `t = clamp((m - 0.8) / 0.1, 0, 1)` give the
    // groove factor `k` and the maximum brightening `e`. A face aligned on an axis (every GREEDY
    // face, REQ-E3D-505) has t = 1, so k = 0.62 and e = 0.10; a face at 45° or more oblique has
    // t = 0, so k = 0.38 and e = 0.20.
    float m = max(an.x, max(an.y, an.z));
    float t = clamp((m - 0.8) / 0.1, 0.0, 1.0);
    float k = 0.38 + 0.24 * t;
    float e = 0.20 - 0.10 * t;

    // Axis weight: an axis perpendicular to the face has weight 0 and cuts no groove at all; the
    // two axes of the face's plane have weight 1.
    vec3 w = clamp(4.0 * (1.0 - an), 0.0, 1.0);

    // Distance to the nearest cell edge, as a fraction of a cell: `min_j |p_a - j N| / N`, in
    // [0, 0.5].
    vec3 cells = p / cell;
    vec3 d = abs(cells - round(cells));

    // The groove is the band of 3.5 % of a cell on each side of an edge; the bright edge runs
    // from there to 16 %, where it reaches exactly 1, so the outer transition is continuous
    // without any smoothing. The inner one, at 3.5 %, is smoothed over about one screen pixel —
    // `fwidth` is the screen-space width of `d`, which is what "about one pixel" means here.
    vec3 width = max(fwidth(d), vec3(1e-6));
    vec3 groove = 1.0 - w * (1.0 - k);
    vec3 bright = 1.0 + w * e * max(vec3(0.0), 0.16 - d) / 0.125;
    vec3 outside = smoothstep(0.035 - width, 0.035 + width, d);
    vec3 f = mix(groove, bright, outside);

    // At the crossing of two edges the two factors multiply (0.62² = 0.384): the corner of a cell
    // is darker than a groove, which is intended (REQ-SCN-081, Notes).
    return f.x * f.y * f.z;
}
