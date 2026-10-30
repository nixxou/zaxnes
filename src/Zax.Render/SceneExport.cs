using System.Numerics;
using Zax.Engine3D;
using Zax.Render.Gl;

namespace Zax.Render;

/// <summary>
/// One object of an exported frame: where its vertices land in the scene, before and after the
/// space bending of SPEC-06 REQ-SCN-068.
/// </summary>
/// <param name="Hud">Whether the object belongs to the HUD plane of REQ-SCN-045.</param>
/// <param name="Bent">Whether the curvature reaches the object at all (REQ-SCN-068).</param>
/// <param name="Clipped">Whether the visible rectangle of REQ-SCN-036 applies to it.</param>
/// <param name="Positions">The final scene positions, bending included.</param>
/// <param name="Unbent">The same vertices before bending, which the clipping is judged on.</param>
/// <param name="Normals">The shading normals, transported by the curvature.</param>
public sealed record ExportedObject(
    bool Hud,
    bool Bent,
    bool Clipped,
    IReadOnlyList<Vector3> Positions,
    IReadOnlyList<Vector3> Unbent,
    IReadOnlyList<Vector3> Normals);

/// <summary>
/// The harness the acceptance criterion of REQ-SCN-068 asks for: the scene-space vertices the
/// renderer would place for a frame, computed on the CPU and without a GL context.
/// </summary>
/// <remarks>
/// <para>
/// It reproduces, in the same order, the chain the vertex shader of
/// <see cref="Gl.SceneShaders"/> applies: the animated deformation of REQ-E3D-568 in the
/// object's local frame, the object's own transform — the sprite nudge of REQ-SCN-037 and the
/// HUD plane of REQ-SCN-045 included, through the shared
/// <see cref="ScenePlacement"/> — then the bending of REQ-SCN-067 on the final scene position.
/// </para>
/// <para>
/// It is a verification aid, not a rendering path: the renderer never calls it, and the scene
/// export of a whole frame is out of scope (PLAN §1, AMD-040 REQ-LUA-115).
/// </para>
/// </remarks>
public static class SceneExport
{
    /// <summary>The objects of <paramref name="scene"/> as the renderer would place them.</summary>
    /// <param name="scene">The frame to export.</param>
    /// <param name="view">The camera view of the frame, which the HUD plane needs.</param>
    public static IReadOnlyList<ExportedObject> Objects(SceneState scene, in CameraView view)
    {
        ArgumentNullException.ThrowIfNull(scene);

        List<ExportedObject> objects = [];
        if (!scene.GameRunning)
        {
            return objects;
        }

        SpaceBend bend = scene.Settings.Bend;
        float time = scene.EmulatedTime;

        if (scene.TwoDimensionalMode)
        {
            objects.Add(Export(
                NesPictureTexture.Quad,
                Mat4.Identity,
                deform: null,
                time,
                bend,
                hud: false,
                bent: true,
                clipped: true));
            return objects;
        }

        if (scene.DrawList is { } list)
        {
            Mat4 hudMatrix = MatrixInterop.HudMatrix(view);
            foreach (DrawTile tile in list.Tiles)
            {
                if (tile.Mesh is not { } mesh || mesh.IsEmpty)
                {
                    continue;
                }

                Mat4 model = ScenePlacement.TileModel(tile, hudMatrix, out bool hud);
                objects.Add(Export(mesh, model, tile.Deform, time, bend, hud, bent: !hud, clipped: !hud));
            }
        }

        if (scene.Backdrop.Enabled)
        {
            Vector3 center = SceneConstants.Center + scene.Backdrop.Offset;
            Mat4 model = Mat4.Translation(MatrixInterop.ToVec3(center))
                * Mat4.ScaleMatrix(MatrixInterop.ToVec3(scene.Backdrop.Size));
            objects.Add(Export(
                BoxMesh.Subdivided,
                model,
                deform: null,
                time,
                bend,
                hud: false,
                bent: true,
                clipped: false));
        }

        return objects;
    }

    private static ExportedObject Export(
        TileMesh mesh,
        Mat4 model,
        DeformParameters? deform,
        float time,
        SpaceBend bend,
        bool hud,
        bool bent,
        bool clipped)
    {
        ReadOnlySpan<MeshVertex> vertices = mesh.Vertices;
        List<Vector3> positions = new(vertices.Length);
        List<Vector3> unbent = new(vertices.Length);
        List<Vector3> normals = new(vertices.Length);

        Span<float> cofactors = stackalloc float[9];
        float determinant = MatrixInterop.NormalMatrix(model, cofactors);
        float sign = determinant < 0f ? -1f : 1f;

        foreach (MeshVertex vertex in vertices)
        {
            Vec3 local = vertex.Position;
            if (deform is { } parameters)
            {
                local += parameters.Displacement(vertex.Position, time);
            }

            Vec3 scene = model.TransformPoint(local);
            Vec3 normal = new(
                (cofactors[0] * vertex.Normal.X)
                    + (cofactors[1] * vertex.Normal.Y)
                    + (cofactors[2] * vertex.Normal.Z),
                (cofactors[3] * vertex.Normal.X)
                    + (cofactors[4] * vertex.Normal.Y)
                    + (cofactors[5] * vertex.Normal.Z),
                (cofactors[6] * vertex.Normal.X)
                    + (cofactors[7] * vertex.Normal.Y)
                    + (cofactors[8] * vertex.Normal.Z));
            normal *= sign;

            Vec3 placed = scene;
            if (bent)
            {
                placed = bend.Apply(scene);
                normal = bend.TransformNormal(scene, normal);
            }

            unbent.Add(MatrixInterop.ToVector(scene));
            positions.Add(MatrixInterop.ToVector(placed));
            normals.Add(MatrixInterop.ToVector(normal));
        }

        return new ExportedObject(hud, bent, clipped, positions, unbent, normals);
    }
}
