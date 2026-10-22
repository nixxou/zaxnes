// Portions adapted from 3DNes_Engine (c) Geod Studio, GPL-3.0

namespace Zax.Engine3D;

/// <summary>
/// Follows one shape from frame to frame (REQ-E3D-300). The subjective camera that follows a
/// character uses it (SPEC-06).
/// </summary>
/// <remarks>
/// <para>
/// The state is the last known position, the followed layer, the set of <b>included</b>
/// patterns and the set of <b>excluded</b> ones. Both sets are only emptied when the tracker is
/// restarted. The walk stops at the first shape whose pattern is included; everything it met
/// before, included shapes aside, is excluded for good. For a sprite follow, "everything
/// before" covers all the background shapes of the frame, because the frame orders its shapes
/// background first (REQ-E3D-220); that is the original's rule and it is kept.
/// </para>
/// <para>
/// The tracker is part of the learned state a savestate carries (DEC-R2-T5).
/// </para>
/// </remarks>
public sealed class ShapeTracker
{
    /// <summary>The proximity threshold δ, in pixels (REQ-E3D-300).</summary>
    public const float Threshold = 9f;

    // Membership is a set, order is a list: a run must be reproducible byte for byte
    // (REQ-ARCH-012), and the hash order of reference types is not (DEC-04-Q02).
    private readonly HashSet<EnginePattern> _included = [];
    private readonly HashSet<EnginePattern> _excluded = [];
    private readonly List<EnginePattern> _includedOrder = [];
    private readonly List<EnginePattern> _excludedOrder = [];

    /// <summary>Whether the tracker is following something.</summary>
    public bool Running { get; private set; }

    /// <summary>True when the followed layer is the sprite layer.</summary>
    public bool Sprites { get; private set; }

    /// <summary>The last known position: the centre of the last shape found (REQ-E3D-044).</summary>
    public Vec3 Position { get; private set; }

    /// <summary>
    /// The included patterns, in the order they were added; the tracker stops at the first
    /// shape carrying one.
    /// </summary>
    public IReadOnlyList<EnginePattern> Included => _includedOrder;

    /// <summary>The excluded patterns, in the order they were added; the walk skips them.</summary>
    public IReadOnlyList<EnginePattern> Excluded => _excludedOrder;

    /// <summary>
    /// Starts the tracker on <paramref name="shape"/>: the position is its centre, the layer
    /// is its layer and the included set holds its reference pattern (REQ-E3D-300).
    /// </summary>
    public void Start(FrameShape shape)
    {
        ArgumentNullException.ThrowIfNull(shape);
        ClearSets();
        Position = shape.Center;
        Sprites = !shape.Background;
        if (shape.ReferencePattern is { } pattern)
        {
            Include(pattern);
        }

        Running = true;
    }

    /// <summary>Stops the tracker.</summary>
    public void Stop() => Running = false;

    /// <summary>
    /// Restores a tracker's state, as a savestate load does (DEC-R2-T5).
    /// </summary>
    public void Restore(
        bool running,
        bool sprites,
        Vec3 position,
        IEnumerable<EnginePattern> included,
        IEnumerable<EnginePattern> excluded)
    {
        ArgumentNullException.ThrowIfNull(included);
        ArgumentNullException.ThrowIfNull(excluded);
        Running = running;
        Sprites = sprites;
        Position = position;
        ClearSets();
        foreach (EnginePattern pattern in included)
        {
            Include(pattern);
        }

        foreach (EnginePattern pattern in excluded)
        {
            Exclude(pattern);
        }
    }

    private void ClearSets()
    {
        _included.Clear();
        _excluded.Clear();
        _includedOrder.Clear();
        _excludedOrder.Clear();
    }

    private void Include(EnginePattern pattern)
    {
        if (_included.Add(pattern))
        {
            _includedOrder.Add(pattern);
        }
    }

    private void Exclude(EnginePattern pattern)
    {
        if (_excluded.Add(pattern))
        {
            _excludedOrder.Add(pattern);
        }
    }

    /// <summary>
    /// The shape of <paramref name="frame"/> that continues the followed one, or null
    /// (REQ-E3D-300).
    /// </summary>
    public FrameShape? Track(ConversionFrame frame)
    {
        ArgumentNullException.ThrowIfNull(frame);
        if (!Running)
        {
            return null;
        }

        IReadOnlyList<FrameShape> shapes = frame.Shapes;
        int start = Sprites ? frame.BackgroundShapeCount : 0;
        int end = Sprites ? shapes.Count : frame.BackgroundShapeCount;

        FrameShape? candidate = null;
        FrameShape? found = null;
        float best = Threshold;

        for (int i = start; i < end; i++)
        {
            FrameShape shape = shapes[i];
            EnginePattern? pattern = shape.ReferencePattern;
            if (pattern is null)
            {
                continue;
            }

            if (_included.Contains(pattern))
            {
                // Case 1: everything up to and including this shape that is not included is
                // excluded for good, and so is the current candidate.
                for (int j = i; j >= 0; j--)
                {
                    if (shapes[j].ReferencePattern is { } earlier && !_included.Contains(earlier))
                    {
                        Exclude(earlier);
                    }
                }

                if (candidate?.ReferencePattern is { } previous)
                {
                    Exclude(previous);
                }

                found = shape;
                break;
            }

            if (_excluded.Contains(pattern))
            {
                // Case 2.
                continue;
            }

            float distance = Vec3.Distance(Position, shape.Center);
            if (distance < best)
            {
                // Case 3: a closer shape; the previous candidate is excluded.
                if (candidate?.ReferencePattern is { } previous)
                {
                    Exclude(previous);
                }

                candidate = shape;
                best = distance;
            }
            else
            {
                // Case 4.
                Exclude(pattern);
            }
        }

        if (found is not null)
        {
            Position = found.Center;
            return found;
        }

        if (candidate is null)
        {
            return null;
        }

        Position = candidate.Center;
        if (candidate.ReferencePattern is { } tracked)
        {
            Include(tracked);
        }

        return candidate;
    }
}
