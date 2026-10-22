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
/// asset; the television of AMD-025 is the glTF model the host hands over, and without it the
/// TV box behaves as <see cref="TvBoxKind.None"/>.
/// </remarks>
public sealed class SceneRenderer : ISceneRenderer
{
    private readonly GL _gl;
    private readonly MeshCache _meshes;
    private readonly TextureCache _textures;
    private readonly ImageTextureCache _images;
    private readonly ShadowMap _shadowMap;
    private readonly SkyboxPass _skybox;
    private readonly ISceneAssets? _assets;
    private readonly TvModel? _television;
    private readonly NesPictureTexture _picture;
    private readonly ShaderProgram _scenePass;
    private readonly ShaderProgram _shadowPass;
    private readonly LinearColorBuffer _linear;

    private readonly List<DrawItem> _opaque = [];
    private readonly List<DrawItem> _transparent = [];
    private readonly List<DrawItem> _hud = [];
    private readonly List<DrawItem> _casters = [];
    private readonly List<DrawGroup> _transparentObjects = [];
    private readonly List<DrawGroup> _hudObjects = [];
    private readonly List<TunnelBox> _tunnel = [];
    private readonly List<TvDrawPart> _tvParts = [];

    private ShaderProgram? _tunnelPass;
    private ShaderProgram? _modelPass;
    private bool _disposed;
    private bool _shadowEnabled;
    private bool _mirroredWinding;
    private bool _mirroredNormal;

    /// <summary>Creates the renderer for an existing, current GL 3.3 core context.</summary>
    /// <param name="gl">The GL API of the host context.</param>
    /// <param name="assets">
    /// Where the skybox cubemaps of a user-supplied folder come from, or null when the host has
    /// none (REQ-SCN-057: the renderer never reads files itself).
    /// </param>
    /// <param name="television">
    /// The television model of AMD-025, already read from its <c>.glb</c>, or null when the
    /// file is missing — the TV box setting then draws nothing.
    /// </param>
    public SceneRenderer(GL gl, ISceneAssets? assets = null, TvModel? television = null)
    {
        ArgumentNullException.ThrowIfNull(gl);
        _gl = gl;
        _assets = assets;
        _television = television;
        _meshes = new MeshCache(gl);
        _textures = new TextureCache(gl);
        _images = new ImageTextureCache(gl, assets);

        // AMD-025: the model's textures live inside its own file, so they are handed to the
        // cache by name instead of being looked up in an asset folder.
        if (television is not null)
        {
            for (int index = 0; index < television.Images.Count; index++)
            {
                _images.Register(TvModel.TextureName(index), television.Images[index]);
            }
        }

        _shadowMap = new ShadowMap(gl, SceneConstants.ShadowMapSize);
        _skybox = new SkyboxPass(gl, assets);
        _picture = new NesPictureTexture(gl);
        _scenePass = ShaderProgram.Create(gl, "scene", SceneShaders.SceneVertex, SceneShaders.SceneFragment);
        _shadowPass = ShaderProgram.Create(gl, "shadow", SceneShaders.ShadowVertex, SceneShaders.ShadowFragment);
        _linear = new LinearColorBuffer(gl);
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
        _modelPass?.Dispose();
        _tunnelPass?.Dispose();
        _shadowPass.Dispose();
        _scenePass.Dispose();
        _picture.Dispose();
        _linear.Dispose();
        _skybox.Dispose();
        _shadowMap.Dispose();
        _images.Dispose();
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
        _tunnel.Clear();
        _tvParts.Clear();

        // REQ-SCN-047: outside a running game the 3D scene is not displayed at all.
        if (!scene.GameRunning)
        {
            return;
        }

        // AMD-009 (Q-022): in 2D mode the core's pixel frame is shown on the NES plane instead
        // of a recomposition of the tile records.
        if (scene.TwoDimensionalMode && CollectPictureQuad(scene, view))
        {
            CollectDecoration(scene, view);
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

        CollectDecoration(scene, view);

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
    /// The tunnel of REQ-SCN-053 and the television of AMD-025, recomputed from the clip
    /// margins at every rendered frame.
    /// </summary>
    /// <remarks>
    /// AMD-025: with no model file there is no television at all, and the setting behaves as
    /// <see cref="TvBoxKind.None"/> — the tunnel alone would be a frame floating in the void.
    /// </remarks>
    private void CollectDecoration(SceneState scene, in CameraView view)
    {
        if (scene.Settings.TvBox == TvBoxKind.None || _television is not { } television)
        {
            return;
        }

        // The tunnel of REQ-SCN-053, clipped to what the cabinet hides (AMD-025).
        VisibleRect footprint = television.Footprint(scene.Clipping);
        foreach (TunnelBox wall in TvBox.Tunnel(scene.Clipping))
        {
            _tunnel.Add(wall.ClampTo(footprint));
        }

        Mat4 model = television.Placement(scene.Clipping);
        foreach (TvModelPart part in television.Parts)
        {
            if (_meshes.Get(part.Mesh) is not { } resident)
            {
                continue;
            }

            bool receives = part.ReceiveShadow && scene.Settings.Shadows;
            _tvParts.Add(new TvDrawPart(resident, model, part.Material, receives));

            // AMD-025, AMD-021 R2-014: the buttons fill the shadow map, the cabinet does not.
            if (part.CastShadow && scene.Settings.Shadows)
            {
                _casters.Add(new DrawItem
                {
                    Mesh = resident,
                    Model = model,
                    Alpha = 1f,
                    BoundsMin = part.Mesh.BoundsMin,
                    BoundsMax = part.Mesh.BoundsMax,
                });
            }
        }

        _ = view;
    }

    /// <summary>
    /// The textured quad of the 2D mode (AMD-009): the core's 256 × 240 picture mapped on the
    /// NES plane of REQ-E3D-410, shaded like any other front-facing tile.
    /// </summary>
    private bool CollectPictureQuad(SceneState scene, in CameraView view)
    {
        ReadOnlySpan<byte> picture = scene.NesPicture.Span;
        uint texture = _picture.Upload(picture);
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
            TextureSize = new Vector2(NesPictureTexture.Width, NesPictureTexture.Height),
            TexTilting = Vector2.One,
            Alpha = 1f,
            // Nothing else is in the scene in 2D mode, so the quad neither casts nor receives.
            ReceiveShadow = false,
            Clip = true,
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

        bool hud = tile.Container == SceneContainer.Interface;
        if (hud)
        {
            // REQ-SCN-045: the HUD lives in a plane attached to the camera.
            model = hudMatrix * model;
        }

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
            Texture = tile.Texture is { } texture ? _textures.Get(texture) : 0u,
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
            Deform = tile.Deform,
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
        GpuMesh? gpu = _meshes.Get(BoxMesh.Unit);
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
            BoundsMin = BoxMesh.Unit.BoundsMin,
            BoundsMax = BoxMesh.Unit.BoundsMax,
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

        Vector3 min = new(float.MaxValue);
        Vector3 max = new(float.MinValue);
        foreach (DrawItem caster in _casters)
        {
            Vector3 point = MatrixInterop.ToVector(caster.Model.TransformPoint(Vec3.Zero));
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
        // face can no longer shadow itself: with a 1024-texel map over the whole scene, one
        // texel spans more than the 0.02 unit bias of REQ-SCN-031, and front faces produce
        // visible shadow acne on every slanted surface. The bias and the normal offset of
        // REQ-SCN-031 are still applied on top.
        _gl.Enable(EnableCap.CullFace);
        _gl.CullFace(TriangleFace.Front);
        SetWinding(false, force: true);

        _shadowPass.Use();
        _shadowPass.Set("uLightViewProjection", volume.ViewProjection);
        _shadowPass.Set("uTime", scene.EmulatedTime);

        foreach (DrawItem caster in _casters)
        {
            // Q-069: a mirrored caster winds the other way round, so the far side of the solid
            // is only selected once the winding is corrected.
            SetWinding(Determinant(caster.Model) < 0f);
            _shadowPass.Set("uModel", MatrixInterop.ToNumerics(caster.Model));
            SetDeformation(_shadowPass, caster.Deform);
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
        _linear.BeginPass(target.Width, target.Height);
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

        // REQ-SCN-058: the skybox replaces the clear, behind everything else. It is drawn even
        // when the game is not running, because only the 3D scene is hidden then
        // (REQ-SCN-047).
        Statistics.SkyboxPass = _skybox.Draw(scene, view);

        if (_opaque.Count == 0
            && _transparent.Count == 0
            && _hud.Count == 0
            && _tunnel.Count == 0
            && _tvParts.Count == 0)
        {
            _linear.ResolveTo(target);
            return;
        }

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

        _scenePass.Use();
        BindCommonUniforms(scene, view, volume);

        // Q-069: the sign lives in the program, which starts at 0 — it has to be written at
        // least once per frame before anything is drawn.
        SetNormalSign(false, force: true);

        // 1. Opaque objects.
        _scenePass.Set("uLightDirection", scene.Light.Direction);
        foreach (DrawItem item in _opaque)
        {
            DrawItemWithMaterial(item, opaque: true);
        }

        Statistics.OpaqueTiles = _opaque.Count;

        // The tiles may have left the winding reversed (Q-069); the decoration that follows is
        // never mirrored.
        SetWinding(false);

        // 1b. The decorative TV box, still opaque (REQ-SCN-053, REQ-SCN-054, REQ-SCN-058).
        if (_tunnel.Count > 0)
        {
            DrawTunnel(scene, view);
        }

        if (_tvParts.Count > 0)
        {
            DrawTvModels(scene, view, volume);
        }

        // 2. Transparent objects, farthest first, each with a depth-only pass first
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

        // 3. HUD, after the transparent objects, lit in the camera frame (REQ-SCN-029,
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
    /// The five tunnel boxes of REQ-SCN-053: one flat colour equal to the background colour of
    /// REQ-SCN-046, unlit, shadowless, and not cut by the visible rectangle.
    /// </summary>
    private void DrawTunnel(SceneState scene, in CameraView view)
    {
        if (_meshes.Get(BoxMesh.Unit) is not { } box)
        {
            return;
        }

        ShaderProgram program = _tunnelPass ??= ShaderProgram.Create(
            _gl, "tunnel", ModelShaders.TunnelVertex, ModelShaders.TunnelFragment);
        program.Use();
        program.Set("uViewProjection", view.ViewProjection);

        // R2-009: the shaders of the scene write linear colours.
        program.Set("uColor", SceneColor.ToLinear(scene.BackgroundColor));

        foreach (TunnelBox wall in _tunnel)
        {
            Vector3 size = wall.Size;
            if (size.X <= 0f || size.Y <= 0f || size.Z <= 0f)
            {
                continue;
            }

            Mat4 model = Mat4.Translation(MatrixInterop.ToVec3(wall.Center))
                * Mat4.ScaleMatrix(MatrixInterop.ToVec3(size));
            program.Set("uModel", MatrixInterop.ToNumerics(model));
            Draw(box);
            Statistics.TunnelBoxes++;
        }
    }

    /// <summary>
    /// The television of AMD-025: the metallic-roughness materials of its glTF file, lit by the
    /// scene light, the cabinet without any shadow and the buttons with one.
    /// </summary>
    private void DrawTvModels(SceneState scene, in CameraView view, ShadowVolume volume)
    {
        ShaderProgram program = _modelPass ??= ShaderProgram.Create(
            _gl, "tv-model", ModelShaders.Vertex, ModelShaders.Fragment);
        program.Use();
        program.Set("uViewProjection", view.ViewProjection);
        program.Set("uEye", view.Eye);
        program.Set("uAmbient", SceneColor.AmbientLinear);
        program.Set("uLightDirection", scene.Light.Direction);
        program.Set(
            "uLightLinear",
            SceneColor.ToLinear(scene.Light.Color) * SceneColor.ToLinear(scene.Settings.LightIntensity));
        program.Set("uBaseColor", 0);
        program.Set("uNormalMap", 1);
        program.Set("uMetallicRoughness", 2);
        program.Set("uEmissive", 3);

        // R2-014: the shadow of the buttons, read with the parameters of REQ-SCN-031.
        program.Set("uShadowMap", 5);
        program.Set("uLightViewProjection", volume.ViewProjection);
        program.Set(
            "uShadowBias",
            volume.HasContent ? ShadowBias(volume, _shadowMap.Size) / volume.DepthRange : 0f);
        program.Set("uNormalOffset", SceneConstants.ShadowNormalOffset);
        program.Set("uShadowStrength", SceneColor.ShadowFactor);
        _gl.ActiveTexture(TextureUnit.Texture5);
        _gl.BindTexture(TextureTarget.Texture2D, _shadowMap.DepthTexture);

        Span<float> normal = stackalloc float[9];
        bool cull = scene.Settings.CullBackFaces;
        bool culling = cull;
        bool mirroredNow = false;
        foreach (TvDrawPart part in _tvParts)
        {
            TvMaterial material = part.Material;
            program.Set("uModel", MatrixInterop.ToNumerics(part.Model));
            MatrixInterop.NormalMatrix(part.Model, normal);
            program.SetMatrix3("uNormalMatrix", normal);

            // A placement with a negative determinant mirrors the model: the triangles come
            // out the other way round and the cofactor normal points inwards.
            bool mirrored = Determinant(part.Model) < 0f;
            program.Set("uNormalSign", mirrored ? -1f : 1f);

            // glTF 2.0 §3.7.4: a double-sided material draws both faces.
            bool wanted = cull && !material.DoubleSided;
            if (wanted != culling)
            {
                if (wanted)
                {
                    _gl.Enable(EnableCap.CullFace);
                }
                else
                {
                    _gl.Disable(EnableCap.CullFace);
                }

                culling = wanted;
            }

            if (culling)
            {
                SetWinding(mirrored);
                mirroredNow = mirrored;
            }

            program.Set("uBaseColorFactor", material.BaseColor);
            program.Set("uAlphaCutout", material.AlphaCutout ? 1 : 0);
            program.Set("uAlphaCutoff", material.AlphaCutoff);
            program.Set("uReceiveShadow", part.ReceiveShadow && volume.HasContent ? 1 : 0);
            program.Set("uMetallic", material.Metallic);
            program.Set("uRoughness", material.Roughness);
            program.Set("uNormalScale", material.NormalScale);
            program.Set("uEmissiveColor", material.EmissiveColor);
            program.Set("uHasNormalMap", _images.Has(material.Normal) ? 1 : 0);
            program.Set("uHasMetallicRoughness", _images.Has(material.MetallicRoughness) ? 1 : 0);
            program.Set("uHasEmissive", _images.Has(material.Emissive) ? 1 : 0);

            _gl.ActiveTexture(TextureUnit.Texture0);
            _gl.BindTexture(TextureTarget.Texture2D, _images.Get(material.Diffuse));
            _gl.ActiveTexture(TextureUnit.Texture1);
            _gl.BindTexture(TextureTarget.Texture2D, _images.Get(material.Normal));
            _gl.ActiveTexture(TextureUnit.Texture2);
            _gl.BindTexture(TextureTarget.Texture2D, _images.Get(material.MetallicRoughness));
            _gl.ActiveTexture(TextureUnit.Texture3);
            _gl.BindTexture(TextureTarget.Texture2D, _images.Get(material.Emissive));

            _gl.ActiveTexture(TextureUnit.Texture0);
            Draw(part.Mesh);
            Statistics.TvParts++;
        }

        if (cull)
        {
            _gl.Enable(EnableCap.CullFace);
            if (mirroredNow)
            {
                SetWinding(false);
            }
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
    /// coarser than that (Q-032).
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
        => MathF.Max(
            SceneConstants.ShadowDepthBias,
            SceneConstants.ShadowBiasTexels * volume.TexelSize(mapSize));

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
        _scenePass.Set("uPaletteTable", item.PaletteTable);
        _scenePass.Set("uTextureSize", item.TextureSize);
        _scenePass.Set("uTexOffset", item.TexOffset);
        _scenePass.Set("uTexTilting", item.TexTilting);
        _scenePass.Set("uFlatColor", item.FlatColor);
        _scenePass.Set("uAlpha", opaque ? 1f : item.Alpha);
        _scenePass.Set("uClip", item.Clip ? 1 : 0);
        _scenePass.Set("uReceiveShadow", _shadowEnabled && item.ReceiveShadow ? 1 : 0);
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

    /// <summary>One resolved mesh of the television model, ready to draw.</summary>
    private readonly record struct TvDrawPart(
        GpuMesh Mesh,
        Mat4 Model,
        TvMaterial Material,
        bool ReceiveShadow);

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
