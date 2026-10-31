using System.Text.Json;
using Zax.Nes;
using Zax.Profiles;
using Zax.Render;

namespace Zax.Harness.Tests;

/// <summary>
/// The mid-run commands and the per-frame log of <c>render --rom</c>: the soft reset of AMD-044
/// (REQ-CORE-122, REQ-LOOP-072), the rollback of AMD-042 (REQ-LOOP-069 to 071, REQ-LOOP-030 to
/// 032), the real-frame numbering the <c>--press</c> script keeps across both, and the camera and
/// <c>PData</c> the capture exports (AMD-045 REQ-SCN-069, SPEC-05 §4.6).
/// </summary>
public sealed class RomRunTests
{
    /// <summary>
    /// A program that counts, in RAM, how many times it has been entered through the reset
    /// vector: <c>SEI</c>, the APU frame IRQ inhibited, <c>INC $0201</c>, then an endless loop.
    /// It is the home-made ROM the acceptance criterion of REQ-CORE-122 describes.
    /// </summary>
    private static byte[] ResetCountingRom => CaptureHarness.Rom(
        0x78,                   // SEI
        0xA9, 0x40,             // LDA #$40
        0x8D, 0x17, 0x40,       // STA $4017   frame counter: IRQ inhibited
        0xEE, 0x01, 0x02,       // INC $0201   resets seen
        0x4C, 0x09, 0x80);      // JMP $8009

    /// <summary>
    /// The same preamble, then a loop that waits for each vblank and increments <c>$0300</c>:
    /// a counter in RAM that advances once per emulated frame, so that a rollback shows up as a
    /// counter that went backwards.
    /// </summary>
    private static byte[] FrameCountingRom => CaptureHarness.Rom(
        0x78,                   // SEI
        0xA9, 0x40,             // LDA #$40
        0x8D, 0x17, 0x40,       // STA $4017
        0xA9, 0x00,             // LDA #$00
        0x8D, 0x00, 0x20,       // STA $2000   PPUCTRL = 0, no NMI
        0xAD, 0x02, 0x20,       // LDA $2002   wait for vblank
        0x10, 0xFB,             // BPL -5      back to the read
        0xEE, 0x00, 0x03,       // INC $0300   frames seen
        0x4C, 0x0B, 0x80);      // JMP $800B

    /// <summary>
    /// <c>PData[1]</c> counts the frame hooks, <c>PData[2]</c> counts the <c>Start</c> hooks, and
    /// the camera is moved once, on the session frame 100.
    /// </summary>
    private const string CountingScript = """
        function Start()
            SetPData(1, GetPData(1) + 1)
        end

        function Update()
            SetPData(0, GetPData(0) + 1)
            if frame == 100 then
                Camera.Mode = 1
                Camera.Distance = 300
            end
        end
        """;

    // ------------------------------------------------------------- soft reset

    [Fact] // REQ-CORE-122, REQ-CORE-006, REQ-LOOP-072
    public void A_soft_reset_runs_the_reset_vector_again_without_clearing_ram()
    {
        // 10 real frames, the RESET button pressed after real frame 6. A hard reset would clear
        // the work RAM and leave the counter at 1; the soft reset keeps it, so it reads 2.
        SyntheticScene capture = Run(runFrames: 10, control: new RomRunControl
        {
            SoftResetAt = 6,
            FrameLogPath = Log(),
        });

        Assert.Equal(2, capture.Core!.ReadMemory(0x0201));
    }

    [Fact] // REQ-CORE-006 (amended by AMD-044), REQ-LOOP-072
    public void A_soft_reset_restarts_the_cores_frame_counter()
    {
        SyntheticScene capture = Run(runFrames: 10, control: new RomRunControl
        {
            SoftResetAt = 6,
            FrameLogPath = Log(),
        });

        JsonElement[] lines = Lines(capture);

        // Real frames 0 to 6 ran one emulated frame each, so the counter reads 7 after the last
        // of them; the reset applied just after it sends the counter back to 0 and real frame 7
        // is the core's frame 1 again.
        Assert.Equal(10, lines.Length);
        Assert.Equal(7, lines[6].GetProperty("coreFrame").GetInt64());
        Assert.Equal(1, lines[7].GetProperty("coreFrame").GetInt64());
        Assert.Equal(3, lines[9].GetProperty("coreFrame").GetInt64());
    }

    [Fact] // REQ-LOOP-072, REQ-ARCH-031
    public void The_press_script_keeps_counting_real_frames_across_a_soft_reset()
    {
        // A held from real frame 4 to real frame 8. The reset after real frame 6 moves the
        // core's counter, not the harness's own count, so frames 7 and 8 still hold A.
        SyntheticScene capture = Run(runFrames: 10, press: "4-8:a", control: new RomRunControl
        {
            SoftResetAt = 6,
            FrameLogPath = Log(),
        });

        JsonElement[] lines = Lines(capture);

        Assert.Equal(
            [0, 0, 0, 0, 1, 1, 1, 1, 1, 0],
            [.. lines.Select(static line => line.GetProperty("input").GetInt32())]);
        Assert.Equal(
            Enumerable.Range(0, 10),
            lines.Select(static line => line.GetProperty("frame").GetInt32()));
    }

    [Fact] // REQ-LOOP-072
    public void A_soft_reset_does_not_relaunch_the_profile()
    {
        // The acceptance criterion of REQ-LOOP-072: PData is not put back to zero and no End or
        // Start hook runs. PData[1] therefore counts all ten frame hooks and PData[2] the single
        // Start of the launch.
        SyntheticScene capture = Run(
            runFrames: 10,
            script: CountingScript,
            control: new RomRunControl { SoftResetAt = 6 });

        Assert.Equal(10f, capture.PersistentData![0]);
        Assert.Equal(1f, capture.PersistentData[1]);
    }

    // --------------------------------------------------------------- rollback

    [Fact] // REQ-LOOP-030, REQ-LOOP-032, REQ-LOOP-069
    public void A_rollback_restores_the_core_state_of_the_second_to_last_snapshot()
    {
        // Snapshots at the start of the real frames whose core counter is a multiple of 60, so
        // at 0, 60 and 120. The second-to-last is the one of the core's frame 60; the four real
        // frames left after the rollback bring the counter to 64.
        SyntheticScene capture = Rolled();

        JsonElement[] lines = Lines(capture);
        Assert.Equal(130, lines.Length);
        Assert.Equal(126, lines[125].GetProperty("coreFrame").GetInt64());
        Assert.Equal(61, lines[126].GetProperty("coreFrame").GetInt64());
        Assert.Equal(64, lines[^1].GetProperty("coreFrame").GetInt64());
    }

    [Fact] // REQ-LOOP-032, REQ-CORE-121
    public void A_rollback_takes_the_work_ram_back_with_the_core()
    {
        // The ROM counts the frames it has seen in RAM. Without the rollback the counter is
        // close to the 130 frames that ran; with it, it goes back to what it was around the
        // core's frame 60 and resumes from there.
        int rolled = Rolled().Core!.ReadMemory(0x0300);
        int straight = Run(runFrames: 130, image: FrameCountingRom).Core!.ReadMemory(0x0300);

        // The vblank loop misses the few frames of the PPU warm-up, so the counter trails the
        // frame count by a handful; what the rollback has to show is the drop of about 60.
        Assert.InRange(straight, 110, 130);
        Assert.InRange(rolled, 45, 80);
        Assert.InRange(straight - rolled, 50, 70);
    }

    [Fact] // REQ-LOOP-069, REQ-LOOP-070
    public void A_rollback_restores_the_sixty_four_pdata_values()
    {
        // The snapshot of the core's frame 60 was taken after 60 frame hooks, so PData[1] was 60
        // then; the four real frames that follow the rollback bring it to 64, and not to 130.
        // The Start hook is not called again (REQ-LOOP-070).
        SyntheticScene capture = Rolled();

        Assert.Equal(64f, capture.PersistentData![0]);
        Assert.Equal(1f, capture.PersistentData[1]);
    }

    [Fact] // REQ-LOOP-069, REQ-LOOP-070, REQ-SCN-024, REQ-SCN-073
    public void A_rollback_restores_the_camera_of_the_snapshot()
    {
        // The script switches to FPV and pushes the distance to 300 on the session frame 100,
        // after the restored snapshot was taken. The rollback has to undo both: the mode goes
        // back to TPV and the FPV setpoint to the 0 of the launch (REQ-SCN-012).
        SceneCamera rolled = Rolled().Scene.Camera;
        SceneCamera straight = Run(
            runFrames: 130,
            image: FrameCountingRom,
            script: CountingScript).Scene.Camera;

        Assert.Equal(CameraMode.Fpv, straight.Mode);
        Assert.Equal(300f, straight.Setpoints(CameraMode.Fpv).Distance);

        Assert.Equal(CameraMode.Tpv, rolled.Mode);
        Assert.Equal(0f, rolled.Setpoints(CameraMode.Fpv).Distance);
        Assert.Equal(SceneConstants.ResetDistance(rolled.Fov), rolled.Current.Distance, 3);
    }

    [Fact] // REQ-LOOP-031, REQ-LOOP-072
    public void A_soft_reset_empties_the_rollback_history()
    {
        // Both commands on the same run: the reset after real frame 100 drops the snapshots of
        // the frames 0 and 60, so the rollback asked for at real frame 125 finds a history built
        // from scratch — one snapshot, the one of the core's frame 0 taken at real frame 101 —
        // and restores it (REQ-LOOP-032, notes).
        SyntheticScene capture = Run(
            runFrames: 130,
            image: FrameCountingRom,
            control: new RomRunControl
            {
                SoftResetAt = 100,
                RollbackAt = 125,
                FrameLogPath = Log(),
            });

        JsonElement[] lines = Lines(capture);
        Assert.Equal(25, lines[125].GetProperty("coreFrame").GetInt64());
        Assert.Equal(1, lines[126].GetProperty("coreFrame").GetInt64());
    }

    [Fact] // REQ-LOOP-030
    public void The_harness_history_keeps_the_depth_and_the_spacing_of_the_loop()
    {
        // The copy the harness holds of the host's history must not drift from it: same depth,
        // same spacing, same step.
        Assert.Equal(30, HarnessRollbackHistory.Capacity);
        Assert.Equal(60, HarnessRollbackHistory.IntervalFrames);
        Assert.True(HarnessRollbackHistory.IsSnapshotFrame(0));
        Assert.True(HarnessRollbackHistory.IsSnapshotFrame(540));
        Assert.False(HarnessRollbackHistory.IsSnapshotFrame(541));

        HarnessRollbackHistory history = new();
        HarnessRollbackSnapshot[] snapshots =
            [.. Enumerable.Range(0, 40).Select(_ => new HarnessRollbackSnapshot([], [], default))];
        foreach (HarnessRollbackSnapshot snapshot in snapshots)
        {
            history.Capture(snapshot);
        }

        Assert.Equal(30, history.Count);

        // REQ-LOOP-032 step 2: the second-to-last one comes back and both it and the last one
        // leave the history.
        Assert.Same(snapshots[38], history.TakeRollbackTarget());
        Assert.Equal(28, history.Count);
    }

    // --------------------------------------------------------- the frame log

    [Fact] // REQ-ARCH-031
    public void The_frame_log_is_one_json_object_per_real_frame_on_its_own_lf_line()
    {
        string path = Log();
        Run(runFrames: 5, control: new RomRunControl { FrameLogPath = path });

        string text = File.ReadAllText(path);
        Assert.DoesNotContain('\r', text);
        Assert.EndsWith("\n", text, StringComparison.Ordinal);

        string[] lines = text.Split('\n', StringSplitOptions.RemoveEmptyEntries);
        Assert.Equal(5, lines.Length);
        for (int n = 0; n < lines.Length; n++)
        {
            using JsonDocument document = JsonDocument.Parse(lines[n]);
            JsonElement line = document.RootElement;
            Assert.Equal(n, line.GetProperty("frame").GetInt32());
            Assert.Equal(0, line.GetProperty("input").GetInt32());
            Assert.Equal(n + 1, line.GetProperty("coreFrame").GetInt64());
            Assert.True(line.GetProperty("tiles").GetInt32() >= 0);
            Assert.Equal(16, line.GetProperty("tileHash").GetString()!.Length);
        }
    }

    [Fact] // REQ-ARCH-012, REQ-ARCH-031
    public void The_same_replay_logs_the_same_tile_hashes()
    {
        string first = Log();
        string second = Log();
        Run(runFrames: 8, image: FrameCountingRom, control: new RomRunControl { FrameLogPath = first });
        Run(runFrames: 8, image: FrameCountingRom, control: new RomRunControl { FrameLogPath = second });

        Assert.Equal(File.ReadAllText(first), File.ReadAllText(second));
    }

    [Fact] // REQ-ARCH-031
    public void The_tile_hash_is_the_fnv_1a_64_of_the_records_dump_frame_writes()
    {
        // An empty frame leaves the hash at the FNV-1a 64 offset basis, which pins the
        // initialisation down.
        Assert.Equal((0, 14695981039346656037UL), FrameLog.Measure([]));

        TileRecord empty = new(0UL, 0UL, 8, 16, 0, false, false, false, 0, true);
        TileRecord drawn = empty with { Plane0 = 0x00000000000000FFUL };

        // Only the records that draw something count, and every field '--dump-frame' writes
        // feeds the hash.
        Assert.Equal(0, FrameLog.Measure([empty, empty]).NonEmptyTiles);
        Assert.Equal(1, FrameLog.Measure([empty, drawn]).NonEmptyTiles);
        Assert.Equal(FrameLog.Measure([empty, drawn]).Hash, FrameLog.Measure([empty, drawn]).Hash);
        Assert.NotEqual(FrameLog.Measure([empty, drawn]).Hash, FrameLog.Measure([drawn, empty]).Hash);
        Assert.NotEqual(
            FrameLog.Measure([drawn]).Hash,
            FrameLog.Measure([drawn with { MirrorH = true }]).Hash);
        Assert.NotEqual(
            FrameLog.Measure([drawn]).Hash,
            FrameLog.Measure([drawn with { Background = false }]).Hash);
    }

    // ------------------------------------------- what render.json carries now

    [Fact] // REQ-SCN-069, REQ-SCN-006, REQ-SCN-011
    public void The_capture_reports_the_camera_of_the_frame_it_shows()
    {
        SyntheticScene capture = Run(runFrames: 4);

        using JsonDocument document = JsonDocument.Parse(
            "{ " + RenderExport.CameraJson(capture.Scene.Camera) + " }");
        JsonElement camera = document.RootElement.GetProperty("camera");

        Assert.Equal("Tpv", camera.GetProperty("mode").GetString());
        Assert.Equal(70f, camera.GetProperty("fov").GetSingle());

        // AMD-045 REQ-SCN-069: d₀ = 132.7964 / tan(FOV / 2) is 189.65 at the default FOV, to the
        // ±0.05 the amendment allows, and the setpoint distance of the launch is that same
        // value, bounded to [0, 512].
        Assert.Equal(189.65f, camera.GetProperty("framedDistance").GetSingle(), 0.05f);
        Assert.Equal(189.65f, camera.GetProperty("setpointDistance").GetSingle(), 0.05f);
        Assert.Equal(
            [0.81f, 0.81f, 0f],
            [.. camera.GetProperty("angles").EnumerateArray().Select(static a => a.GetSingle())]);
    }

    [Fact] // REQ-SCN-069
    public void The_reported_framed_distance_follows_the_field_of_view()
    {
        // The three acceptance values of AMD-045 REQ-SCN-069, to the ±0.05 it allows, read back
        // through the export.
        foreach ((float fov, float expected) in new[] { (45f, 320.60f), (70f, 189.65f), (95f, 121.69f) })
        {
            SceneCamera camera = new() { Fov = fov };
            using JsonDocument document = JsonDocument.Parse(
                "{ " + RenderExport.CameraJson(camera) + " }");

            Assert.Equal(
                expected,
                document.RootElement.GetProperty("camera").GetProperty("framedDistance").GetSingle(),
                0.05f);
        }
    }

    [Fact] // REQ-LUA-070, REQ-LUA-073
    public void The_capture_reports_the_sixty_four_pdata_values_as_lua_reads_them()
    {
        SyntheticScene capture = Run(runFrames: 4, script: CountingScript);

        using JsonDocument document = JsonDocument.Parse(
            "{ \"x\": 0" + RenderExport.PdataJson(capture.PersistentData) + " }");
        float[] exported =
            [.. document.RootElement.GetProperty("pdata").EnumerateArray().Select(static v => v.GetSingle())];

        // 64 entries, the first one being PData[1] — the slot SetPData(0, …) addresses
        // (REQ-LUA-070) — and the second PData[2].
        Assert.Equal(64, exported.Length);
        Assert.Equal(4f, exported[0]);
        Assert.Equal(1f, exported[1]);
        Assert.Equal(0f, exported[63]);
    }

    [Fact] // REQ-ARCH-031
    public void A_synthetic_capture_reports_a_camera_and_no_pdata()
    {
        // There is no Lua runtime behind a synthetic frame, so the export says nothing about
        // PData rather than claiming 64 zeros.
        Assert.Equal(string.Empty, RenderExport.PdataJson(null));
        Assert.Contains("\"camera\"", RenderExport.CameraJson(new SceneCamera()), StringComparison.Ordinal);
    }

    // ------------------------------------------------------------- validation

    [Theory] // REQ-ARCH-031
    [InlineData(-1)]
    [InlineData(9)]
    [InlineData(20)]
    public void A_command_outside_the_run_is_refused(int at)
    {
        // The frame a command names has to run, and a real frame has to follow it: the converted
        // frames in flight are dropped by both commands (REQ-E3D-281), so a capture taken right
        // after one would have nothing to show.
        Assert.Throws<ArgumentException>(
            () => new RomRunControl { SoftResetAt = at }.Validate(10));
        Assert.Throws<ArgumentException>(
            () => new RomRunControl { RollbackAt = at }.Validate(10));
    }

    [Fact] // REQ-ARCH-031
    public void The_last_frame_but_one_is_accepted()
    {
        new RomRunControl { SoftResetAt = 8, RollbackAt = 8 }.Validate(10);
        Assert.False(new RomRunControl().KeepsHistory);
        Assert.True(new RomRunControl { RollbackAt = 1 }.KeepsHistory);
        Assert.True(new RomRunControl().IsEmpty);
    }

    // ------------------------------------------------------------------ setup

    /// <summary>The 130-frame run of the rollback tests, rolled back after real frame 125.</summary>
    private static SyntheticScene Rolled() => Run(
        runFrames: 130,
        image: FrameCountingRom,
        script: CountingScript,
        control: new RomRunControl { RollbackAt = 125, FrameLogPath = Log() });

    private static SyntheticScene Run(
        int runFrames,
        byte[]? image = null,
        string? script = null,
        string? press = null,
        RomRunControl? control = null)
        => CaptureHarness.Capture(
            script is null ? GameProfile.CreateEmpty() : CaptureHarness.WithScript(script),
            runFrames,
            image: image ?? ResetCountingRom,
            press: press,
            control: control);

    /// <summary>A fresh path for a frame log; the file is left behind for a failed run.</summary>
    private static string Log()
        => Path.Combine(Path.GetTempPath(), $"zax-frames-{Guid.NewGuid():N}.jsonl");

    /// <summary>The lines of the run's frame log, parsed.</summary>
    private static JsonElement[] Lines(SyntheticScene capture)
        => [.. capture.FrameLog!.Lines.Select(static line =>
        {
            using JsonDocument document = JsonDocument.Parse(line);
            return document.RootElement.Clone();
        })];
}
