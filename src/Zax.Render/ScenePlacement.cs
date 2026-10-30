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
        // REQ-SCN-037: the renderer is the sole owner of the ±0.1 sprite depth nudge, applied
        // in the object's local frame, before the scale and the rotation, exactly once.
        float nudge = tile.Sprite
            ? tile.BehindBackground ? SceneConstants.SpriteNudge : -SceneConstants.SpriteNudge
            : 0f;
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
}
