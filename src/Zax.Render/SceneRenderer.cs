using System.Numerics;
using Zax.Engine3D;
using Zax.Profiles;
using Zax.Render.Assets;
using Zax.Render.Gl;
using Silk.NET.OpenGL;

namespace Zax.Render;

/// <summary>
/// The OpenGL 3.3 core renderer of SPEC-06: shadow map, background or skybox, opaque objects,
/// sorted transparent objects, then the HUD (REQ-SCN-058), all inside a framebuffer the host
/// provides (REQ-SCN-057).
/// </summary>
/// <remarks>
/// Not thread-safe, and bound to the GL context that was current when it was created. The
/// skyboxes of REQ-SCN-050 and REQ-SCN-051 are procedural (AMD-024 REQ-PUB-003) and need no
/// asset, so the scene is the game's image and its background and nothing else (AMD-031).
/// </remarks>
public sealed class SceneRenderer : ISceneRenderer
{
    private readonly GL _gl;
    private readonly MeshCache _meshes;
    private readonly TextureCache _textures;
    private readonly ShadowMap _shadowMap;
    private readonly SkyboxPass _skybox;
    private readonly ISceneAssets? _assets;
    private readonly NesPictureTexture _picture;
    private readonly ShaderProgram _scenePass;
    private readonly ShaderProgram _shadowPass;
    private readonly LinearColorBuffer _linear;
    private readonly OcclusionPass _occlusion;
    private readonly ReflectionPass _reflections;

    private readonly List<DrawItem> _opaque = [];
    private readonly List<DrawItem> _transparent = [];
    private readonly List<DrawItem> _hud = [];
    private readonly List<DrawItem> _casters = [];
    private readonly List<DrawGroup> _transparentObjects = [];
    private readonly List<DrawGroup> _hudObjects = [];

    private bool _disposed;
    private bool _shadowEnabled;
    private bool _pixelShadow;

    /// <summary>
    /// Whether this frame is the 2D-mode rectangle of AMD-046 §2 rather than a 3D scene. The
    /// screen passes of AMD-055 never touch it (REQ-SCN-083, scope).
    /// </summary>
    private bool _pictureQuad;
    private bool _mirroredWinding;
    private bool _mirroredNormal;
    private SpaceBend _bend = SpaceBend.None;

    /// <summary>Creates the renderer for an existing, current GL 3.3 core context.</summary>
    /// <param name="gl">The GL API of the host context.</param>
    /// <param name="assets">
    /// Where the skybox cubemaps of a user-supplied folder come from, or null when the host has
    /// none (REQ-SCN-057: the renderer never reads files itself).
    /// </param>
    public SceneRenderer(GL gl, ISceneAssets? assets = null)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        _assets = assets;
        _meshes = new MeshCache(gl);
        _textures = new TextureCache(gl);
        _shadowMap = new ShadowMap(gl, SceneConstants.ShadowMapSize);
        _skybox = new SkyboxPass(gl, assets);
        _picture = new NesPictureTexture(gl);
        _scenePass = ShaderProgram.Create(gl, "scene", SceneShaders.SceneVertex, SceneShaders.SceneFragment);
        _shadowPass = ShaderProgram.Create(gl, "shadow", SceneShaders.ShadowVertex, SceneShaders.ShadowFragment);
        _linear = new LinearColorBuffer(gl);
        _occlusion = new OcclusionPass(gl);
        _reflections = new ReflectionPass(gl);

        // AMD-034 §2: the size the hardware granted is known from here on, whether or not a
        // shadow pass ever runs, so the host can report a fallback once.
        Statistics.ShadowMapSize = _shadowMap.Size;
        Statistics.RequestedShadowMapSize = _shadowMap.Requested;
    }

    /// <inheritdoc/>
    public RenderStatistics Statistics { get; } = new();

    /// <summary>The camera view the last rendered frame used.</summary>
    public CameraView LastView { get; private set; }

    /// <inheritdoc/>
    public void Render(in RenderTarget target, SceneState scene)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        ArgumentNullException.ThrowIfNull(scene);
        target.Validate();

        Statistics.BeginFrame();
        _meshes.BeginFrame();
        _textures.BeginFrame();

        float aspect = target.Width / (float)target.Height;
        CameraView view = scene.Camera.BuildView(scene.Clipping, aspect);
        LastView = view;

        Collect(scene, view);
        ShadowVolume volume = RenderShadowMap(scene, view);
        RenderScene(target, scene, view, volume);

        Statistics.ResidentMeshes = _meshes.Count;
        Statistics.MeshUploads = _meshes.Uploads;
        Statistics.ResidentTextures = _textures.Count;
        _meshes.Collect();
        _textures.Collect();
    }

    /// <inheritdoc/>
    public void InvalidateCaches()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        _meshes.Clear();
        _textures.Clear();
    }

    /// <inheritdoc/>
    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _shadowPass.Dispose();
        _scenePass.Dispose();
        _picture.Dispose();
        _linear.Dispose();
        _occlusion.Dispose();
        _reflections.Dispose();
        _skybox.Dispose();
        _shadowMap.Dispose();
        _textures.Dispose();
        _meshes.Dispose();
    }

    // ----------------------------------------------------------------- collection

    /// <summary>
    /// Turns the tile list of REQ-E3D-641 into the four buckets of REQ-SCN-058, applying the
    /// renderer-side sprite nudge of REQ-SCN-037 and the material rules of REQ-SCN-040/041.
    /// </summary>
    private void Collect(SceneState scene, in CameraView view)
    {
        _opaque.Clear();
        _transparent.Clear();
        _hud.Clear();
        _casters.Clear();
        _pictureQuad = false;

        // REQ-SCN-047: outside a running game the 3D scene is not displayed at all.
        if (!scene.GameRunning)
        {
            return;
        }

        // AMD-009 (Q-022): in 2D mode the core's pixel frame is shown on the NES plane instead
        // of a recomposition of the tile records.
        if (scene.TwoDimensionalMode && CollectPictureQuad(scene, view))
        {
            _pictureQuad = true;
            return;
        }

        FrameDrawList? list = scene.DrawList;
        if (list is not null)
        {
            Mat4 hudMatrix = MatrixInterop.HudMatrix(view);
            Occlusion occlusion = Occlusion.For(scene, view);
            foreach (DrawTile tile in list.Tiles)
            {
                CollectTile(tile, scene, view, hudMatrix, occlusion);
            }
        }

        if (scene.Backdrop.Enabled)
        {
            CollectBackdrop(scene, view);
        }

        // AMD-055 REQ-SCN-084: which opaque objects are wide enough in the xz plane of the scene
        // frame to carry a floor surface. Only worth measuring when the reflections are on.
        if (scene.Settings.Style.EffectiveReflections && !_pictureQuad)
        {
            MarkFloorSurfaces(scene.Settings.SceneScale);
        }

        // AMD-012 (Q-020 revised): the depth pass and the sort of the transparent objects are per
        // whole object, so the faces between the tiles of one translucent object stay invisible.
        // REQ-SCN-040: they are drawn after the opaque ones, farthest first.
        GroupByObject(_transparent, _transparentObjects, view.Eye);
        _transparentObjects.Sort(static (a, b) => b.ViewDepth.CompareTo(a.ViewDepth));

        // REQ-SCN-044 blends the HUD "as in REQ-SCN-040", so its objects get the same treatment;
        // their order stays the list order, since the HUD is not sorted by distance.
        GroupByObject(_hud, _hudObjects, view.Eye);
    }

    /// <summary>
    /// The extent half of the floor definition of AMD-055 REQ-SCN-084: an opaque object whose
    /// bounding box in the <c>xz</c> plane of the scene frame, <b>before curvature</b>, measures
    /// at least <c>6 × scene scale</c> units in both dimensions may carry floor surfaces.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The objects are the instances of AMD-012 — the runs of tiles that share one shape
    /// instance, which SPEC-04B REQ-E3D-641 delivers consecutively. The backdrop has no
    /// instance, so it stands as an object of its own and is measured on its own box
    /// (<b>Q-093</b>): REQ-SCN-084 names « une instance », and the backdrop is none, but the
    /// scope bullet of the requirement lists what is never a floor surface and leaves it out.
    /// With its default size the question is moot, its box measuring 256 × 1 units in <c>xz</c>.
    /// </para>
    /// <para>
    /// <see cref="DrawItem.GrowBounds"/> transforms the local box by the placement matrix alone,
    /// which is exactly the « avant courbure » the requirement asks for.
    /// </para>
    /// </remarks>
    // Q-093
    private void MarkFloorSurfaces(float sceneScale)
    {
        float threshold = SceneConstants.FloorExtent * sceneScale;
        int start = 0;
        while (start < _opaque.Count)
        {
            object? instance = _opaque[start].Instance;
            int end = start + 1;
            while (end < _opaque.Count
                && instance is not null
                && ReferenceEquals(_opaque[end].Instance, instance))
            {
                end++;
            }

            Vector3 min = new(float.MaxValue);
            Vector3 max = new(float.MinValue);
            for (int index = start; index < end; index++)
            {
                _opaque[index].GrowBounds(ref min, ref max);
            }

            bool wide = max.X - min.X >= threshold && max.Z - min.Z >= threshold;
            for (int index = start; index < end; index++)
            {
                DrawItem item = _opaque[index];
                item.FloorExtent = wide;
                _opaque[index] = item;
            }

            start = end;
        }
    }

    /// <summary>
    /// Cuts <paramref name="items"/> into the objects of AMD-012 — the runs of tiles that share
    /// one shape instance, which SPEC-04B REQ-E3D-641 delivers consecutively — and measures each
    /// one by the distance from the eye to the centre of its transformed bounding box.
    /// </summary>
    private static void GroupByObject(List<DrawItem> items, List<DrawGroup> groups, Vector3 eye)
    {
        groups.Clear();
        int start = 0;
        while (start < items.Count)
        {
            object? instance = items[start].Instance;
            int end = start + 1;
            while (end < items.Count
                && instance is not null
                && ReferenceEquals(items[end].Instance, instance))
            {
                end++;
            }

            Vector3 min = new(float.MaxValue);
            Vector3 max = new(float.MinValue);
            for (int index = start; index < end; index++)
            {
                items[index].GrowBounds(ref min, ref max);
            }

            groups.Add(new DrawGroup(start, end - start, Vector3.Distance((min + max) * 0.5f, eye)));
            start = end;
        }
    }

    /// <summary>
    /// The 2D-mode rectangle of AMD-046 REQ-SCN-074 (AMD-009 Q-022): the core's 256 × 240
    /// picture mapped on the NES plane of REQ-E3D-410, lit like a tile (REQ-SCN-028, albedo =
    /// the colour of the pixel) and with the universal-background pixels of AMD-050
    /// REQ-SCN-076 dropped.
    /// </summary>
    private bool CollectPictureQuad(SceneState scene, in CameraView view)
    {
        ReadOnlySpan<byte> picture = scene.NesPicture.Span;
        uint texture = _picture.Upload(picture, scene.NesOpacityMask.Span);
        if (texture == 0 || _meshes.Get(NesPictureTexture.Quad) is not { } mesh)
        {
            return false;
        }

        _opaque.Add(new DrawItem
        {
            Mesh = mesh,
            Model = Mat4.Identity,
            ColorMode = 1,
            Texture = texture,

            // AMD-050 REQ-SCN-076: the pixels the core's opacity mask (REQ-CORE-049) marks as
            // carrying no opaque pixel are rejected, here in the opaque pass — the rectangle
            // stays out of the transparent class of REQ-SCN-040 and the result does not depend
            // on video.anti_alias.
            Cutout = true,
            TextureSize = new Vector2(NesPictureTexture.Width, NesPictureTexture.Height),
            TexTilting = Vector2.One,
            Alpha = 1f,
            // REQ-SCN-074: the 2D-mode rectangle casts no shadow and receives none, whatever
            // the setting says. It is never added to the casters either.
            ReceiveShadow = false,
            Clip = true,
            // REQ-SCN-074: the rectangle is the game scene in 2D mode, so the curvature of
            // REQ-SCN-067 reaches it like the tiles it stands in for — unless « Bend 2D Mode »
            // is off, which puts every one of its vertices back at its flat position.
            Bend = scene.Settings.Bend2dMode,
            BoundsMin = NesPictureTexture.Quad.BoundsMin,
            BoundsMax = NesPictureTexture.Quad.BoundsMax,
        });

        _ = view;
        return true;
    }

    private void CollectTile(
        in DrawTile tile,
        SceneState scene,
        in CameraView view,
        Mat4 hudMatrix,
        in Occlusion occlusion)
    {
        if (tile.Mesh is not { } mesh || mesh.IsEmpty)
        {
            return;
        }

        GpuMesh? gpu = _meshes.Get(mesh);
        if (gpu is not { } resident)
        {
            return;
        }

        // AMD-051 REQ-E3D-644: a texture the driver refuses to allocate leaves the tile out of
        // the frame — never an exception, and never drawn with somebody else's texture. The
        // other patterns of the frame are drawn normally and the next frame is produced.
        uint handle = 0;
        if (tile.Texture is { } tileTexture)
        {
            handle = _textures.Get(tileTexture);
            if (handle == 0)
            {
                Statistics.UnavailableTextures++;
                return;
            }
        }

        Mat4 model = ScenePlacement.TileModel(tile, hudMatrix, out bool hud);

        // REQ-SCN-022: in FPV, an opaque tile of another shape that crosses the eye-to-target
        // segment is drawn transparent at 0.5 for this frame only.
        MaterialClass material = tile.Material;
        bool occluder = material == MaterialClass.Opaque
            && !hud
            && occlusion.Hides(tile, mesh, model);
        if (occluder)
        {
            material = MaterialClass.Transparent;
            Statistics.Occluders++;
        }

        DrawItem item = new()
        {
            Instance = tile.Instance,
            BoundsMin = mesh.BoundsMin,
            BoundsMax = mesh.BoundsMax,
            Mesh = resident,
            Model = model,
            ColorMode = tile.TextureFormat == TextureFormat.Direct ? 1 : 0,
            Texture = handle,
            TextureSize = tile.Texture is { } size
                ? new Vector2(size.Width, size.Height)
                : Vector2.One,
            PaletteTable = tile.PaletteTable,
            TexOffset = new Vector2(tile.TexOffset.X, tile.TexOffset.Y),
            TexTilting = new Vector2(tile.TexTilting.X, tile.TexTilting.Y),
            Alpha = occluder
                ? SceneConstants.OccluderAlpha
                : hud || material == MaterialClass.Transparent ? tile.Alpha : 1f,
            // AMD-021 R2-010 (REQ-SCN-032 "should" becomes "must not"): a tile rendered in
            // transparency — the FPV occluder of REQ-SCN-022 included — and a HUD tile receive
            // no shadow at all, whatever the flags of their motif and of their layer.
            ReceiveShadow = tile.ReceiveShadow && !hud && material != MaterialClass.Transparent,
            Clip = !hud,
            Hud = hud,
            // REQ-SCN-068: the converted tiles and sprites bend, the HUD stays flat.
            Bend = !hud,
            Deform = tile.Deform,

            // AMD-055 REQ-SCN-081: the grid reaches the opaque and the transparent tiles, in
            // the local mesh frame of each tile, but never the HUD (REQ-SCN-044, -045).
            GridCell = hud ? 0f : scene.Settings.Style.EffectiveGridCell,
            GridInSceneFrame = false,

            // REQ-SCN-082: the pixel shadows are for the faces of opaque tile meshes only.
            // A transparent tile receives no shadow at all (REQ-SCN-032), so the material of
            // this frame is what decides, the FPV occluder of REQ-SCN-022 included.
            PixelShadow = !hud && material == MaterialClass.Opaque,
            BuildDepth = tile.BuildDepth,
        };

        // A tile only casts while it is rendered opaque (REQ-SCN-032, REQ-SCN-040).
        if (tile.CastShadow && !occluder)
        {
            _casters.Add(item);
        }

        switch (material)
        {
            case MaterialClass.ShadowOnly:
                // REQ-SCN-041 style 2: not drawn, but always casts its shadow.
                break;

            case MaterialClass.Transparent:
                _transparent.Add(item);
                break;

            case MaterialClass.Interface:
                _hud.Add(item);
                break;

            default:
                _opaque.Add(item);
                break;
        }
    }

    private void CollectBackdrop(SceneState scene, in CameraView view)
    {
        BackdropState backdrop = scene.Backdrop;

        // REQ-SCN-068: the backdrop is subdivided, at least 32 × 32 cells per face, so that the
        // curvature shows inside a face instead of only tilting it.
        GpuMesh? gpu = _meshes.Get(BoxMesh.Subdivided);
        if (gpu is not { } resident)
        {
            return;
        }

        Vector3 center = SceneConstants.Center + backdrop.Offset;
        Mat4 model = Mat4.Translation(MatrixInterop.ToVec3(center))
            * Mat4.ScaleMatrix(MatrixInterop.ToVec3(backdrop.Size));

        _opaque.Add(new DrawItem
        {
            Mesh = resident,
            Model = model,
            ColorMode = 2,
            FlatColor = SceneColor.ToVector(backdrop.Color),
            TextureSize = Vector2.One,
            TexTilting = Vector2.One,
            Alpha = 1f,
            // REQ-SCN-048: never casts, receives when asked, never clipped by R.
            ReceiveShadow = scene.Settings.Shadows && backdrop.ReceiveShadow,
            Clip = false,
            // REQ-SCN-068: the backdrop bends with the scene it stands behind.
            Bend = true,

            // AMD-055 REQ-SCN-081 (amending AMD-049 REQ-SCN-075): the grid reaches the faces
            // of the box, in the scene frame, modulating the albedo c before the lighting.
            GridCell = scene.Settings.Style.EffectiveGridCell,
            GridInSceneFrame = true,
            BoundsMin = BoxMesh.Subdivided.BoundsMin,
            BoundsMax = BoxMesh.Subdivided.BoundsMax,
        });

        _ = view;
    }

    // ----------------------------------------------------------------- shadow pass

    private ShadowVolume RenderShadowMap(SceneState scene, in CameraView view)
    {
        if (!scene.Settings.Shadows || _casters.Count == 0)
        {
            return ShadowVolume.Empty;
        }

        _shadowMap.Resize(scene.Settings.ShadowMapSize);
        Statistics.ShadowMapSize = _shadowMap.Size;
        Statistics.RequestedShadowMapSize = _shadowMap.Requested;

        SpaceBend bend = scene.Settings.Bend;
        Vector3 min = new(float.MaxValue);
        Vector3 max = new(float.MinValue);
        foreach (DrawItem caster in _casters)
        {
            Vec3 anchor = caster.Model.TransformPoint(Vec3.Zero);

            // REQ-SCN-068: the shadow pass draws the bent geometry, so the volume has to be
            // fitted on the bent anchors or the casters at the edges fall outside the map.
            Vector3 point = MatrixInterop.ToVector(caster.Bend ? bend.Apply(anchor) : anchor);
            min = Vector3.Min(min, point);
            max = Vector3.Max(max, point);
        }

        // The tiles are at most 48 units across in their local frame (REQ-E3D-523), so padding
        // the anchors is enough and costs one matrix instead of one per vertex.
        min -= new Vector3(64f);
        max += new Vector3(64f);

        ShadowVolume volume = ShadowVolume.Fit(min, max, view.Eye, scene.Light.Direction);
        if (!volume.HasContent)
        {
            return ShadowVolume.Empty;
        }

        _shadowMap.BeginPass();
        _gl.Enable(EnableCap.DepthTest);
        _gl.DepthFunc(DepthFunction.Less);
        _gl.DepthMask(true);
        _gl.Disable(EnableCap.Blend);

        // Q-025: only the back faces of the casters fill the map. The tile meshes are closed
        // (REQ-E3D-505), so the stored depth is the far side of each object and a lit front
        // face can no longer shadow itself: over a whole scene one texel still spans more than
        // the 0.02 unit bias of REQ-SCN-031 — about 0.1 unit on the 4096-texel map of
        // AMD-034 §2 — and front faces produce visible shadow acne on every slanted surface.
        // The bias and the normal offset of REQ-SCN-031 are still applied on top.
        _gl.Enable(EnableCap.CullFace);
        _gl.CullFace(TriangleFace.Front);
        SetWinding(false, force: true);

        _shadowPass.Use();
        _shadowPass.Set("uLightViewProjection", volume.ViewProjection);
        _shadowPass.Set("uTime", scene.EmulatedTime);
        SetBendConstants(_shadowPass, bend);

        foreach (DrawItem caster in _casters)
        {
            // Q-069: a mirrored caster winds the other way round, so the far side of the solid
            // is only selected once the winding is corrected.
            SetWinding(Determinant(caster.Model) < 0f);
            _shadowPass.Set("uModel", MatrixInterop.ToNumerics(caster.Model));
            SetDeformation(_shadowPass, caster.Deform);
            _shadowPass.Set("uBend", caster.Bend && !bend.IsIdentity ? 1 : 0);
            Draw(caster.Mesh);
        }

        SetWinding(false);

        Statistics.ShadowCasters = _casters.Count;
        Statistics.ShadowPass = true;
        return volume;
    }

    // ----------------------------------------------------------------- scene passes

    private void RenderScene(
        in RenderTarget target,
        SceneState scene,
        in CameraView view,
        ShadowVolume volume)
    {
        // AMD-021 R2-009: the scene is drawn into the renderer's own linear buffer, so that the
        // blending of the transparent objects and of the HUD happens on linear colours, and the
        // whole image is encoded to sRGB once, on the way into the host framebuffer.
        // AMD-032 §2: that buffer is multisampled when anti-aliasing is on, and resolved before
        // the sRGB pass reads it. The shadow map keeps its own single-sample target.
        // AMD-055 REQ-SCN-084: the floor mask is an extra attachment, so it is only asked for
        // on the frames whose reflections are on. In 2D mode the rectangle of AMD-046 §2 is the
        // whole scene and is never a floor surface, so the frame needs none.
        bool floorMask = scene.Settings.Style.EffectiveReflections && !_pictureQuad;
        _linear.BeginPass(
            target.Width, target.Height, scene.Settings.AntiAliasSamples, floorMask);
        Statistics.AntiAliasSamples = _linear.Samples;
        _gl.Disable(EnableCap.ScissorTest);
        _gl.Disable(EnableCap.Blend);
        _gl.Enable(EnableCap.DepthTest);
        _gl.DepthFunc(DepthFunction.Less);
        _gl.DepthMask(true);
        _gl.ColorMask(true, true, true, true);

        // REQ-SCN-046: the image is cleared with the background colour, without any lighting.
        Vector3 background = SceneColor.ToLinear(scene.BackgroundColor);
        _gl.ClearColor(background.X, background.Y, background.Z, 1f);
        _gl.ClearDepth(1.0);
        _gl.Clear(ClearBufferMask.ColorBufferBit | ClearBufferMask.DepthBufferBit);

        // REQ-SCN-084: « no floor here » everywhere the opaque pass does not say otherwise.
        _linear.ClearFloorMask();

        // REQ-SCN-058: the skybox replaces the clear, behind everything else. It is drawn even
        // when the game is not running, because only the 3D scene is hidden then
        // (REQ-SCN-047).
        Statistics.SkyboxPass = _skybox.Draw(scene, view);

        if (_opaque.Count == 0 && _transparent.Count == 0 && _hud.Count == 0)
        {
            _linear.ResolveTo(target);
            return;
        }

        RestoreCulling(scene);

        _scenePass.Use();
        BindCommonUniforms(scene, view, volume);

        // Q-069: the sign lives in the program, which starts at 0 — it has to be written at
        // least once per frame before anything is drawn.
        SetNormalSign(false, force: true);

        // 1. Opaque objects. They are the only pass that writes the floor mask of REQ-SCN-084:
        // the transparent objects and the HUD are drawn after the reflections and carry none.
        _linear.SetFloorMaskTarget(true);
        _scenePass.Set("uLightDirection", scene.Light.Direction);
        foreach (DrawItem item in _opaque)
        {
            DrawItemWithMaterial(item, opaque: true);
        }

        _linear.SetFloorMaskTarget(false);
        Statistics.OpaqueTiles = _opaque.Count;

        // The tiles may have left the winding reversed (Q-069).
        SetWinding(false);

        // 2. AMD-055 REQ-SCN-058 amended: in the Enhanced style, the screen passes come here —
        // after the opaque objects, before the sorted transparent ones. What occludes and what
        // is darkened follows from that order and from it alone (REQ-SCN-080): the skybox is
        // behind everything and never occludes, the transparent objects and the HUD are drawn
        // after and are neither occluders nor occluded.
        // The 2D-mode rectangle of AMD-046 §2 must not be modified by the effect
        // (REQ-SCN-083, scope), and in 2D mode it is the whole scene, so the pass is skipped.
        if (!_pictureQuad)
        {
            // REQ-SCN-058 amended: the floor reflections come first, then the occlusion — so
            // the occlusion darkens a reflection that has already been laid down, and never the
            // other way round.
            Statistics.ReflectionPass = _reflections.Draw(
                _linear, scene.Settings.Style, view, scene.Settings.SceneScale);
            Statistics.OcclusionPass = _occlusion.Draw(
                _linear, scene.Settings.Style, view.Projection, scene.Settings.SceneScale);
            if (Statistics.ReflectionPass || Statistics.OcclusionPass)
            {
                // The passes left the cull state and the program of their own full-screen
                // triangles behind; the passes that follow rebind their program themselves.
                RestoreCulling(scene);
                _scenePass.Use();
                BindCommonUniforms(scene, view, volume);
                SetNormalSign(false, force: true);
            }
        }

        // 3. Transparent objects, farthest first, each with a depth-only pass first
        // (REQ-SCN-040, AMD-012).
        if (_transparent.Count > 0)
        {
            _scenePass.Use();
            BindCommonUniforms(scene, view, volume);
            _scenePass.Set("uLightDirection", scene.Light.Direction);
            _gl.Enable(EnableCap.Blend);
            _gl.BlendFunc(BlendingFactor.SrcAlpha, BlendingFactor.OneMinusSrcAlpha);
            foreach (DrawGroup group in _transparentObjects)
            {
                DrawBlended(_transparent, group);
            }

            _gl.Disable(EnableCap.Blend);
            Statistics.TransparentTiles = _transparent.Count;
            Statistics.TransparentObjects = _transparentObjects.Count;
        }

        // 4. HUD, after the transparent objects, lit in the camera frame (REQ-SCN-029,
        // REQ-SCN-044).
        if (_hud.Count > 0)
        {
            _scenePass.Use();
            BindCommonUniforms(scene, view, volume);
            _scenePass.Set("uLightDirection", view.HudLightDirection(scene.Light.Direction));
            _gl.Enable(EnableCap.Blend);
            _gl.BlendFunc(BlendingFactor.SrcAlpha, BlendingFactor.OneMinusSrcAlpha);
            foreach (DrawGroup group in _hudObjects)
            {
                DrawBlended(_hud, group);
            }

            _gl.Disable(EnableCap.Blend);
            Statistics.HudTiles = _hud.Count;
        }

        _gl.DepthMask(true);
        _gl.DepthFunc(DepthFunction.Less);
        SetWinding(false);
        _linear.ResolveTo(target);
    }

    /// <summary>
    /// Puts back the face culling the frame is drawn with. The screen pass of AMD-055 draws a
    /// full-screen triangle with culling off, so this runs again after it.
    /// </summary>
    private void RestoreCulling(SceneState scene)
    {
        if (scene.Settings.CullBackFaces)
        {
            // The scene frame is left-handed and the clip position mirrors z, so the faces the
            // engine winds per REQ-E3D-505 come out clockwise (REQ-SCN-001 notes).
            _gl.Enable(EnableCap.CullFace);
            _gl.CullFace(TriangleFace.Back);
            SetWinding(false, force: true);
        }
        else
        {
            _gl.Disable(EnableCap.CullFace);
        }
    }

    /// <summary>
    /// Selects the winding the rasterizer takes for front faces: clockwise for an ordinary
    /// placement, counter-clockwise for a mirrored one (Q-069).
    /// </summary>
    /// <remarks>
    /// A placement whose determinant is negative — the profile of "Zelda II" scales its ground
    /// slab by −1 on z — reverses the order of every triangle in screen space. Left alone, the
    /// cull test then keeps exactly the faces REQ-E3D-505 oriented away from the eye: the solid
    /// renders inside-out, which is what put the slab behind the grass it carries. Reversing the
    /// winding for that object restores the surface a mirrored solid still has.
    /// </remarks>
    /// <param name="mirrored">True to wind front faces counter-clockwise.</param>
    /// <param name="force">
    /// True to issue the call even when the cached state already matches, which is what the
    /// entry of a pass does: another pass of the host context may have changed it.
    /// </param>
    private void SetWinding(bool mirrored, bool force = false)
    {
        if (mirrored == _mirroredWinding && !force)
        {
            return;
        }

        _gl.FrontFace(mirrored ? FrontFaceDirection.Ccw : FrontFaceDirection.CW);
        _mirroredWinding = mirrored;
    }

    /// <summary>
    /// Sets the sign the scene shader applies to the cofactor normal: −1 for a mirrored
    /// placement, which would otherwise point the normal inwards (Q-069).
    /// </summary>
    private void SetNormalSign(bool mirrored, bool force = false)
    {
        if (mirrored == _mirroredNormal && !force)
        {
            return;
        }

        _scenePass.Set("uNormalSign", mirrored ? -1f : 1f);
        _mirroredNormal = mirrored;
    }

    /// <summary>Determinant of the upper-left 3 × 3 block of a placement matrix.</summary>
    private static float Determinant(Mat4 m)
        => (m[0, 0] * ((m[1, 1] * m[2, 2]) - (m[1, 2] * m[2, 1])))
            - (m[0, 1] * ((m[1, 0] * m[2, 2]) - (m[1, 2] * m[2, 0])))
            + (m[0, 2] * ((m[1, 0] * m[2, 1]) - (m[1, 1] * m[2, 0])));

    private void BindCommonUniforms(SceneState scene, in CameraView view, ShadowVolume volume)
    {
        // The palette is refreshed first: the upload binds its own texture, so it has to happen
        // before the sampler units are set up (REQ-E3D-565).
        _gl.ActiveTexture(TextureUnit.Texture0);
        if (scene.DrawList is { } list)
        {
            _textures.UploadPalette(list.Palette);
        }

        _scenePass.Set("uViewProjection", view.ViewProjection);
        _scenePass.Set("uTime", scene.EmulatedTime);
        _scenePass.Set("uAmbient", SceneColor.AmbientLinear);
        _scenePass.Set("uShadowStrength", SceneColor.ShadowFactor);

        // REQ-SCN-075 (AMD-049): the backdrop's material is the only view-dependent one, so the
        // eye is bound once per frame like the rest of the light.
        _scenePass.Set("uEye", view.Eye);

        Vector3 light = SceneColor.ToLinear(scene.Light.Color)
            * SceneColor.ToLinear(scene.Settings.LightIntensity);
        _scenePass.Set("uLightLinear", light);

        VisibleRect rect = scene.VisibleRectangle;
        _scenePass.Set("uClipRect", new Vector4(rect.MinX, rect.MinY, rect.MaxX, rect.MaxY));

        _scenePass.Set("uLightViewProjection", volume.ViewProjection);
        _scenePass.Set(
            "uShadowBias",
            volume.HasContent ? ShadowBias(volume, _shadowMap.Size) / volume.DepthRange : 0f);
        _scenePass.Set("uNormalOffset", SceneConstants.ShadowNormalOffset);
        _shadowEnabled = volume.HasContent;

        // AMD-055 REQ-SCN-082: the setting is read once per frame, like the curvature.
        _pixelShadow = scene.Settings.Style.EffectivePixelShadow;

        // REQ-SCN-067: the per-axis constants are the same for every vertex of the frame.
        _bend = scene.Settings.Bend;
        SetBendConstants(_scenePass, _bend);

        _scenePass.Set("uIndexTexture", 0);
        _scenePass.Set("uColorTexture", 1);
        _scenePass.Set("uPalette", 2);
        _scenePass.Set("uShadowMap", 3);

        _gl.ActiveTexture(TextureUnit.Texture2);
        _gl.BindTexture(TextureTarget.Texture2D, _textures.Palette);
        _gl.ActiveTexture(TextureUnit.Texture3);
        _gl.BindTexture(TextureTarget.Texture2D, _shadowMap.DepthTexture);
    }

    /// <summary>
    /// The depth bias of the shadow test, in scene units: the 0.02 unit of REQ-SCN-031, raised
    /// to <see cref="SceneConstants.ShadowBiasTexels"/> texels of the current map when a texel is
    /// coarser than that (Q-032, amended by AMD-034 §2 — one texel, never under 0.02 unit).
    /// </summary>
    /// <remarks>
    /// Q-025 fills the map with back faces only. A tile mesh is closed on itself (REQ-E3D-505),
    /// so the wall that closes a tile against its neighbour is a back face of the light as well
    /// and goes into the map, even though it sits <b>inside</b> the solid the two tiles form. A
    /// receiver one texel away from a joint then reads that inner wall instead of the far side of
    /// the solid and darkens: the seam of REQ-SCN-065. The error is proportional to the texel, so
    /// the bias has to be too — R0's answer to Q-025 puts the aim (no visible acne) above the
    /// 0.02 unit figure.
    /// </remarks>
    private static float ShadowBias(ShadowVolume volume, int mapSize)
        => SceneConstants.ShadowBias(volume.TexelSize(mapSize));

    private void DrawItemWithMaterial(in DrawItem item, bool opaque)
    {
        _scenePass.Set("uModel", MatrixInterop.ToNumerics(item.Model));
        Span<float> normal = stackalloc float[9];
        float determinant = MatrixInterop.NormalMatrix(item.Model, normal);
        _scenePass.SetMatrix3("uNormalMatrix", normal);

        // Q-069: a placement with a negative determinant mirrors the object. Its triangles come
        // out the other way round and its cofactor normal points inwards, so both are corrected
        // here; otherwise the solid renders inside-out. Almost no object is mirrored, so both
        // corrections are cached and cost nothing on an ordinary scene.
        bool mirrored = determinant < 0f;
        SetNormalSign(mirrored);
        SetWinding(mirrored);

        _scenePass.Set("uColorMode", item.ColorMode);
        _scenePass.Set("uCutout", item.Cutout ? 1 : 0);
        _scenePass.Set("uPaletteTable", item.PaletteTable);
        _scenePass.Set("uTextureSize", item.TextureSize);
        _scenePass.Set("uTexOffset", item.TexOffset);
        _scenePass.Set("uTexTilting", item.TexTilting);
        _scenePass.Set("uFlatColor", item.FlatColor);
        _scenePass.Set("uAlpha", opaque ? 1f : item.Alpha);
        _scenePass.Set("uClip", item.Clip ? 1 : 0);
        _scenePass.Set("uReceiveShadow", _shadowEnabled && item.ReceiveShadow ? 1 : 0);
        _scenePass.Set("uBend", item.Bend && !_bend.IsIdentity ? 1 : 0);

        // AMD-055 REQ-SCN-081: the cell side, 0 where the grid does not apply, and the frame
        // it is evaluated in — the local mesh frame of a tile, the scene frame of the backdrop.
        _scenePass.Set("uGridCell", item.GridCell);
        _scenePass.Set("uGridScene", item.GridInSceneFrame ? 1 : 0);

        // AMD-055 REQ-SCN-082: only the faces of opaque tile meshes read the shadow at the
        // centre of their voxel.
        _scenePass.Set("uPixelShadow", _pixelShadow && item.PixelShadow ? 1 : 0);
        _scenePass.Set("uBuildDepth", (float)item.BuildDepth);

        // AMD-055 REQ-SCN-084: the extent half of the floor test. Only the opaque pass writes
        // the mask attachment at all, so this is the only place the flag can reach it.
        _scenePass.Set("uFloorExtent", opaque && item.FloorExtent ? 1 : 0);
        SetDeformation(_scenePass, item.Deform);

        _gl.ActiveTexture(TextureUnit.Texture0);
        _gl.BindTexture(
            TextureTarget.Texture2D,
            item.ColorMode == 0 && item.Texture != 0 ? item.Texture : _textures.DummyIndexed);
        _gl.ActiveTexture(TextureUnit.Texture1);
        _gl.BindTexture(
            TextureTarget.Texture2D,
            item.ColorMode == 1 && item.Texture != 0 ? item.Texture : _textures.DummyColor);

        Draw(item.Mesh);
    }

    /// <summary>
    /// Draws one blended object with the two passes of REQ-SCN-040: depth only, then colour
    /// where the depth is equal, so that only the nearest surface of the object is visible.
    /// </summary>
    /// <remarks>
    /// "An object" is a whole shape instance, every tile of it at once (AMD-012, revising
    /// Q-020): the first pass lays down the depth of all its tiles before any of them is
    /// blended, so the faces that close each tile mesh on itself (SPEC-04B REQ-E3D-505) at the
    /// joints inside the object are never drawn. Taking one tile as the object instead made
    /// those joints show as a grid through a translucent multi-tile object.
    /// </remarks>
    private void DrawBlended(List<DrawItem> items, DrawGroup group)
    {
        int end = group.Start + group.Count;

        _gl.ColorMask(false, false, false, false);
        _gl.DepthMask(true);
        _gl.DepthFunc(DepthFunction.Less);
        for (int index = group.Start; index < end; index++)
        {
            DrawItemWithMaterial(items[index], opaque: false);
        }

        _gl.ColorMask(true, true, true, true);
        _gl.DepthMask(false);
        _gl.DepthFunc(DepthFunction.Equal);
        for (int index = group.Start; index < end; index++)
        {
            DrawItemWithMaterial(items[index], opaque: false);
        }

        _gl.DepthMask(true);
        _gl.DepthFunc(DepthFunction.Less);
    }

    /// <summary>
    /// Uploads the per-frame constants of the space bending (REQ-SCN-067): the curvature, the
    /// recentring factor and the offset of each of the three axes, and the arc pivot.
    /// </summary>
    private static void SetBendConstants(ShaderProgram program, SpaceBend bend)
    {
        program.Set("uBendK", MatrixInterop.ToVector(bend.Curvature));
        program.Set("uBendF", MatrixInterop.ToVector(bend.Recentring));
        program.Set("uBendH", MatrixInterop.ToVector(bend.Offset));
        program.Set("uBendOn", MatrixInterop.ToVector(bend.Active));
        program.Set("uBendPivot", bend.ArcPivot);
    }

    private void SetDeformation(ShaderProgram program, DeformParameters? deform)
    {
        if (deform is not { } parameters)
        {
            program.Set("uDeform", 0);
            return;
        }

        program.Set("uDeform", 1);
        program.Set("uDeformAmplitude", MatrixInterop.ToVector(parameters.Amplitude));
        program.Set("uDeformSize", MatrixInterop.ToVector(parameters.Size));
        program.Set("uDeformTile", MatrixInterop.ToVector(parameters.TileRelativeToPivot));
        for (int row = 0; row < 3; row++)
        {
            program.SetArray("uDeformSpeed", row, Row(parameters.Speed, row));
            program.SetArray("uDeformPhase", row, Row(parameters.Phase, row));
        }
    }

    private unsafe void Draw(GpuMesh mesh)
    {
        _gl.BindVertexArray(mesh.VertexArray);
        _gl.DrawElements(PrimitiveType.Triangles, mesh.IndexCount, DrawElementsType.UnsignedInt, (void*)0);
        Statistics.DrawCalls++;
        Statistics.Triangles += mesh.IndexCount / 3;
    }

    private static Vector4 Row(Mat4 matrix, int row)
        => new(matrix[row, 0], matrix[row, 1], matrix[row, 2], matrix[row, 3]);

    /// <summary>
    /// One blended object of AMD-012: the slice <c>[Start, Start + Count)</c> of a draw list and
    /// the distance from the eye to the centre of its transformed bounding box.
    /// </summary>
    private readonly record struct DrawGroup(int Start, int Count, float ViewDepth);

    private struct DrawItem
    {
        public GpuMesh Mesh;
        public Mat4 Model;
        public int ColorMode;
        public uint Texture;
        public Vector2 TextureSize;
        public int PaletteTable;
        public Vector2 TexOffset;
        public Vector2 TexTilting;
        public Vector3 FlatColor;
        public float Alpha;
        public bool ReceiveShadow;
        public bool Clip;
        public bool Hud;

        /// <summary>
        /// Whether a texel of alpha 0 rejects the fragment instead of being blended
        /// (REQ-SCN-076): the 2D-mode rectangle and nothing else.
        /// </summary>
        public bool Cutout;

        /// <summary>Whether the space bending of REQ-SCN-068 reaches this item.</summary>
        public bool Bend;

        /// <summary>
        /// The cell side of the voxel grid for this item, 0 when the grid does not apply to it
        /// (AMD-055 REQ-SCN-081): the HUD, the skybox and the 2D-mode rectangle never carry it.
        /// </summary>
        public float GridCell;

        /// <summary>
        /// Whether the grid of this item is evaluated in the scene frame — which is what the
        /// backdrop does — rather than in the local mesh frame of a tile (REQ-SCN-081).
        /// </summary>
        public bool GridInSceneFrame;

        /// <summary>
        /// Whether the pixel shadows of REQ-SCN-082 may reach this item: the faces of opaque
        /// tile meshes, and nothing else.
        /// </summary>
        public bool PixelShadow;

        /// <summary>
        /// The construction depth <c>D</c> of the local mesh frame (REQ-E3D-504), which the
        /// voxel-centre snap of REQ-SCN-082 offsets the <c>z</c> boundaries by.
        /// </summary>
        public int BuildDepth;

        /// <summary>
        /// Whether the object this item belongs to is wide enough in the <c>xz</c> plane of the
        /// scene frame for its upward faces to be floor surfaces (AMD-055 REQ-SCN-084).
        /// </summary>
        public bool FloorExtent;

        public DeformParameters? Deform;

        /// <summary>
        /// The shape instance the tile belongs to (SPEC-04B REQ-E3D-641), the object the depth
        /// pass and the sort of AMD-012 work on. Null for the backdrop and the 2D-mode quad,
        /// which are single objects anyway.
        /// </summary>
        public object? Instance;

        /// <summary>Local bounding box of the mesh (REQ-E3D-640).</summary>
        public Vec3 BoundsMin;

        /// <summary>Local bounding box of the mesh (REQ-E3D-640).</summary>
        public Vec3 BoundsMax;

        /// <summary>Grows a scene-space box by the eight transformed corners of this item.</summary>
        public readonly void GrowBounds(ref Vector3 min, ref Vector3 max)
        {
            for (int corner = 0; corner < 8; corner++)
            {
                Vec3 local = new(
                    (corner & 1) == 0 ? BoundsMin.X : BoundsMax.X,
                    (corner & 2) == 0 ? BoundsMin.Y : BoundsMax.Y,
                    (corner & 4) == 0 ? BoundsMin.Z : BoundsMax.Z);
                Vector3 point = MatrixInterop.ToVector(Model.TransformPoint(local));
                min = Vector3.Min(min, point);
                max = Vector3.Max(max, point);
            }
        }
    }
}
