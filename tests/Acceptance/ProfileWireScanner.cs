namespace Zax.Acceptance.Tests;

/// <summary>
/// An independent reader of the profile wire format, written by the verifier straight from
/// SPEC-03 §3.1, §3.2 and §4, so that the structural counts of §5.5 are checked without
/// going through the implementation's decoder.
/// </summary>
public sealed class ProfileWireScanner
{
    private readonly byte[] _bytes;
    private int _nextKey = 1;

    private ProfileWireScanner(byte[] bytes) => _bytes = bytes;

    /// <summary>Keys seen so far, in order of creation.</summary>
    public HashSet<int> KnownKeys { get; } = [];

    /// <summary>New-object envelopes encountered, by kind.</summary>
    public Dictionary<string, int> NewObjects { get; } = [];

    /// <summary>Back-references encountered, by kind.</summary>
    public Dictionary<string, int> References { get; } = [];

    /// <summary>Non-empty grid cells.</summary>
    public int NonEmptyCells { get; private set; }

    /// <summary>Empty grid cells (the <c>0b 0c</c> group).</summary>
    public int EmptyCells { get; private set; }

    /// <summary>Patterns that carry a grid.</summary>
    public int GriddedPatterns { get; private set; }

    /// <summary>Grids whose cell count equals <c>tSize.x * tSize.y</c>.</summary>
    public int ConsistentGrids { get; private set; }

    /// <summary>Field numbers seen at the root that the schema does not define.</summary>
    public int UnknownRootFields { get; private set; }

    /// <summary>Whether every new key was the next one in sequence.</summary>
    public bool KeysAreSequential { get; private set; } = true;

    private static void NoOp(int from, int to)
    {
    }

    /// <summary>Scans one profile file.</summary>
    public static ProfileWireScanner Scan(byte[] bytes)
    {
        ProfileWireScanner scanner = new(bytes);
        scanner.Root(0, bytes.Length);
        return scanner;
    }

    private static readonly int[] RootFields = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

    private void Root(int from, int to)
    {
        int at = from;
        while (at < to)
        {
            (int number, int wire, int next) = Tag(at);
            if (!RootFields.Contains(number))
            {
                UnknownRootFields++;
            }

            at = Field(number, wire, next, (start, end) =>
            {
                switch (number)
                {
                    case 4:
                    case 5:
                        Envelope(start, end, "pattern", Pattern);
                        break;
                    case 6:
                        Settings(start, end);
                        break;
                }
            });
        }
    }

    private void Settings(int from, int to)
    {
        int at = from;
        while (at < to)
        {
            (int number, int wire, int next) = Tag(at);
            at = Field(number, wire, next, (start, end) =>
            {
                if (number is 14 or 15)
                {
                    RecordedSlot(start, end);
                }
            });
        }
    }

    private void RecordedSlot(int from, int to)
    {
        int at = from;
        while (at < to)
        {
            (int number, int wire, int next) = Tag(at);
            at = Field(number, wire, next, (start, end) =>
            {
                if (number == 3)
                {
                    Pattern(start, end); // direct message, no envelope
                }
            });
        }
    }

    private void Pattern(int from, int to)
    {
        int at = from;
        int cells = 0;
        int tileWidth = 0;
        int tileHeight = 0;
        bool hasGrid = false;

        while (at < to)
        {
            (int number, int wire, int next) = Tag(at);

            if (number == 1 && wire == 3)
            {
                hasGrid = true;
                cells++;
                at = Group(next, (start, end) =>
                {
                    if (start == end)
                    {
                        EmptyCells++;
                        return;
                    }

                    NonEmptyCells++;
                    int inside = start;
                    while (inside < end)
                    {
                        (int n2, int w2, int next2) = Tag(inside);
                        inside = Field(n2, w2, next2, (s, e) =>
                        {
                            if (n2 == 1)
                            {
                                Envelope(s, e, "tile", Cell);
                            }
                        });
                    }
                });
                continue;
            }

            at = Field(number, wire, next, (start, end) =>
            {
                switch (number)
                {
                    case 3:
                        (tileWidth, tileHeight) = IVec2(start, end);
                        break;
                    case 10:
                        Envelope(start, end, "variant", NoOp);
                        break;
                }
            });
        }

        if (hasGrid)
        {
            GriddedPatterns++;
            if (cells == tileWidth * tileHeight)
            {
                ConsistentGrids++;
            }
        }
    }

    private void Cell(int from, int to)
    {
        // Cell.1 = TilePlanes; nothing nested needs a key.
    }

    private (int X, int Y) IVec2(int from, int to)
    {
        int at = from;
        int x = 0;
        int y = 0;
        while (at < to)
        {
            (int number, int wire, int next) = Tag(at);
            if (wire == 0)
            {
                (ulong value, int after) = Varint(next);
                if (number == 1)
                {
                    x = (int)(long)value;
                }
                else if (number == 2)
                {
                    y = (int)(long)value;
                }

                at = after;
            }
            else
            {
                at = Field(number, wire, next, NoOp);
            }
        }

        return (x, y);
    }

    /// <summary>Reads a reference envelope and dispatches the carried message.</summary>
    private void Envelope(int from, int to, string kind, Action<int, int> body)
    {
        int at = from;
        int? existing = null;
        int? created = null;
        (int Start, int End)? payload = null;

        while (at < to)
        {
            (int number, int wire, int next) = Tag(at);
            if (wire == 0)
            {
                (ulong value, int after) = Varint(next);
                if (number == 1)
                {
                    existing = (int)value;
                }
                else if (number == 2)
                {
                    created = (int)value;
                }

                at = after;
                continue;
            }

            int captured = number;
            at = Field(number, wire, next, (start, end) =>
            {
                if (captured == 10)
                {
                    payload = (start, end);
                }
            });
        }

        if (existing is { } key)
        {
            References[kind] = References.GetValueOrDefault(kind) + 1;
            if (!KnownKeys.Contains(key))
            {
                KeysAreSequential = false;
            }

            return;
        }

        NewObjects[kind] = NewObjects.GetValueOrDefault(kind) + 1;
        if (created != _nextKey)
        {
            KeysAreSequential = false;
        }

        KnownKeys.Add(created ?? -1);
        _nextKey++;

        if (payload is { } range)
        {
            body(range.Start, range.End);
        }
    }

    // ------------------------------------------------------------------ wire primitives

    private (int Number, int Wire, int Next) Tag(int at)
    {
        (ulong tag, int next) = Varint(at);
        return ((int)(tag >> 3), (int)(tag & 7), next);
    }

    private (ulong Value, int Next) Varint(int at)
    {
        ulong value = 0;
        int shift = 0;
        while (true)
        {
            byte b = _bytes[at++];
            value |= (ulong)(b & 0x7F) << shift;
            if ((b & 0x80) == 0)
            {
                return (value, at);
            }

            shift += 7;
        }
    }

    private int Field(int number, int wire, int at, Action<int, int> onMessage)
    {
        switch (wire)
        {
            case 0:
                return Varint(at).Next;
            case 1:
                return at + 8;
            case 2:
            {
                (ulong length, int start) = Varint(at);
                int end = start + (int)length;
                onMessage(start, end);
                return end;
            }

            case 3:
                return Group(at, NoOp);
            case 5:
                return at + 4;
            default:
                throw new InvalidDataException($"wire type {wire} on field {number}");
        }
    }

    /// <summary>
    /// Visits a group body, calling <paramref name="inner"/> with the payload bounds and
    /// returning the index just past the EGROUP tag.
    /// </summary>
    private int Group(int at, Action<int, int> inner)
    {
        int start = at;
        int cursor = at;
        while (true)
        {
            int tagAt = cursor;
            (int number, int wire, int next) = Tag(cursor);
            if (wire == 4)
            {
                inner(start, tagAt);
                return next;
            }

            cursor = wire == 3
                ? Group(next, NoOp)
                : Field(number, wire, next, NoOp);
        }
    }
}
