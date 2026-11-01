using Zax.Engine3D;

namespace Zax.Render;

/// <summary>
/// Where one tile of the draw list of SPEC-04B REQ-E3D-641 ends up in the scene, before the
/// space bending of REQ-SCN-068: the transform the engine produced, the renderer's own sprite
/// depth nudge (REQ-SCN-037) and, for a HUD tile, the plane attached to the camera
/// (REQ-SCN-045).
/// </summary>
/// <remarks>
/// It is a class of its own so that the renderer and the export harness of REQ-SCN-068 place a
/// tile the same way by construction, rather than by two transcriptions of the same rules.
/// </remarks>
internal static class ScenePlacement
{
    /// <summary>The scene placement of <paramref name="tile"/>.</summary>
    /// <param name="tile">The tile record of REQ-E3D-641.</param>
    /// <param name="hudMatrix">The HUD plane of REQ-SCN-045, for the current camera.</param>
    /// <param name="hud">True when the tile belongs to the interface container.</param>
    public static Mat4 TileModel(in DrawTile tile, Mat4 hudMatrix, out bool hud)
    {
        float nudge = SpriteNudge(tile);
        Mat4 model = tile.Transform;
        if (nudge != 0f)
        {
            model *= Mat4.Translation(new Vec3(0f, 0f, nudge));
        }

        hud = tile.Container == SceneContainer.Interface;
        if (hud)
        {
            // REQ-SCN-045: the HUD lives in a plane attached to the camera.
            model = hudMatrix * model;
        }

        return model;
    }

    /// <summary>
    /// The depth nudge <c>n</c> of SPEC-06 REQ-SCN-037, as AMD-057 amends it:
    /// <c>0.001 × (i mod 64) − 0.1</c> for a sprite tile in front of the background and
    /// <c>0.001 × (i mod 64) + 0.1</c> for one behind it, where <c>i</c> is the sprite index
    /// of REQ-CORE-043. A background tile is not nudged.
    /// </summary>
    /// <remarks>
    /// The renderer is the sole owner of the nudge (DEC-06-Q4, DEC-R2-T4): it is applied in
    /// the object's local frame, before the scale and the rotation, exactly once. Since the
    /// camera looks down +z, a smaller index gives a smaller <c>z</c> and therefore wins the
    /// depth test: at equal overlap the sprite the NES gives priority to is the one that
    /// shows, and two overlapping sprites no longer flicker. The two halves of an 8 × 16
    /// share one nudge because REQ-CORE-043 numbers the lower half <c>64 + n</c>.
    /// </remarks>
    /// <param name="tile">The tile record of REQ-E3D-641.</param>
    public static float SpriteNudge(in DrawTile tile)
    {
        if (!tile.Sprite)
        {
            return 0f;
        }

        // A sprite shape only ever holds sprite tiles, so the index is the core's; the guard
        // is there for a draw list built by hand with no index at all.
        int index = tile.SpriteIndex < 0 ? 0 : tile.SpriteIndex % 64;
        float step = SceneConstants.SpriteIndexNudge * index;
        return tile.BehindBackground
            ? step + SceneConstants.SpriteNudge
            : step - SceneConstants.SpriteNudge;
    }
}
