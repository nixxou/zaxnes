using Zax.Nes.State;

namespace Zax.Nes.Apu;

/// <summary>
/// The APU of the 2A03: five sound channels, the frame counter and its IRQ, the $4015 status
/// register, the mixer and the resampled output buffer (REQ-CORE-070 to REQ-CORE-074,
/// REQ-CORE-113).
/// </summary>
/// <remarks>
/// <para>
/// Written from the public nesdev documentation ("APU", "APU Pulse", "APU Triangle",
/// "APU Noise", "APU DMC", "APU Frame Counter", "APU Mixer", "DMA") and from the result
/// protocols of the public test ROMs. The two pulse channels, the triangle, the noise and
/// the DMC each own their state in <see cref="PulseChannel"/>, <see cref="TriangleChannel"/>,
/// <see cref="NoiseChannel"/> and <see cref="DmcChannel"/>; this class owns the clocking, the
/// register decode and the output path.
/// </para>
/// <para>
/// <strong>Register writes land after the cycle.</strong> <see cref="NesBus"/> advances the
/// whole machine by one cycle and only then performs the register access, so a write always
/// takes effect after that cycle's frame-counter events. That ordering is what the hardware
/// does and what blargg's <c>10.len_halt_timing</c> and <c>11.len_reload_timing</c> measure;
/// nothing else in this class has to compensate for it.
/// </para>
/// <para>
/// <strong>Expansion audio</strong> (VRC6, VRC7, MMC5, Namco 163, Sunsoft 5B, FDS) is not
/// emulated: REQ-CORE-075 makes it optional and DEC-02-Q5 settles that it is not required.
/// </para>
/// </remarks>
public sealed class Apu2A03
{
    /// <summary>Lowest output sample rate the core accepts (REQ-CORE-070).</summary>
    public const int MinSampleRate = 22_050;

    /// <summary>Highest output sample rate the core accepts (REQ-CORE-070).</summary>
    public const int MaxSampleRate = 96_000;

    /// <summary>Default output sample rate (REQ-CORE-070, SPEC-11 REQ-AUD-001).</summary>
    public const int DefaultSampleRate = 48_000;

    /// <summary>Minimum capacity of the output buffer, in mono samples (REQ-CORE-071).</summary>
    public const int MinBufferSamples = 10_000;

    /// <summary>Largest fine adjustment of the resampling ratio a frontend may ask for (REQ-CORE-070).</summary>
    public const double MaxRatioAdjustment = 0.005;

    /// <summary>Volume one step of a pulse channel contributes (REQ-CORE-073).</summary>
    public const double PulseStep = 0.00752;

    /// <summary>Volume one step of the triangle contributes (REQ-CORE-073).</summary>
    public const double TriangleStep = 0.00851;

    /// <summary>Volume one step of the noise contributes (REQ-CORE-073).</summary>
    public const double NoiseStep = 0.00494;

    /// <summary>Volume one step of the DMC contributes (REQ-CORE-073).</summary>
    public const double DmcStep = 0.00335;

    /// <summary>Output gain applied to the mix, 50 000 / 32 768 (REQ-CORE-073).</summary>
    public const double OutputGain = 1.526;

    /// <summary>
    /// Four-step frame sequencer, NTSC, in CPU cycles after the sequencer reset (nesdev,
    /// "APU Frame Counter"). The last entry closes the sequence and restarts it.
    /// </summary>
    private static readonly int[] NtscFourStep = [7457, 14913, 22371, 29828, 29829, 29830];

    /// <summary>Five-step frame sequencer, NTSC CPU cycles.</summary>
    private static readonly int[] NtscFiveStep = [7457, 14913, 22371, 29829, 37281, 37282];

    /// <summary>Four-step frame sequencer, PAL CPU cycles.</summary>
    private static readonly int[] PalFourStep = [8313, 16627, 24939, 33250, 33251, 33252];

    /// <summary>Five-step frame sequencer, PAL CPU cycles.</summary>
    private static readonly int[] PalFiveStep = [8313, 16627, 24939, 33253, 41565, 41566];

    private readonly object _bufferLock = new();

    private readonly PulseChannel _pulse1 = new(isFirst: true);
    private readonly PulseChannel _pulse2 = new(isFirst: false);
    private readonly TriangleChannel _triangle = new();
    private readonly NoiseChannel _noise = new();
    private readonly DmcChannel _dmc = new();
    private readonly BandLimitedResampler _resampler;

    private float[] _buffer = new float[MinBufferSamples];
    private int _readIndex;
    private int _writeIndex;
    private int _available;

    private RegionTiming _timing = RegionTiming.For(NesRegion.Ntsc);
    private int _sampleRate = DefaultSampleRate;
    private double _speed = 1.0;
    private double _ratioAdjustment;

    private long _cycles;
    private bool _apuCycle;
    private int _frameCounter;
    private int _frameStep;
    private bool _fiveStepMode;
    private bool _irqInhibited;
    private bool _frameIrqFlag;
    private bool _frameIrqPending;
    private int _pendingSequencerReset;
    private byte _readableStatus;
    private int _levelKey = -1;
    private double _level;

    /// <summary>Creates an APU with the default region and sample rate.</summary>
    public Apu2A03()
    {
        _resampler = new BandLimitedResampler(Enqueue);
        ApplyTiming();
        RecomputeRatio();
    }

    /// <summary>Whether the APU is pulling /IRQ low: frame counter IRQ or DMC IRQ.</summary>
    public bool IrqAsserted => _frameIrqFlag || _dmc.IrqFlag;

    /// <summary>
    /// The mixed output level of the five channels on the cycle just stepped, in [−1, 1]
    /// (REQ-CORE-073), before resampling and before the DC blocker.
    /// </summary>
    /// <remarks>
    /// This is the signal the resampler is fed. It is exposed so that the mixer can be
    /// checked against the figures of REQ-CORE-073 without inferring them back from the
    /// filtered output.
    /// </remarks>
    public double MixedLevel => _level;

    /// <summary>Whether the DMC's memory reader wants a byte (nesdev, "DMA").</summary>
    public bool NeedsDmcDma => _dmc.NeedsSampleFetch;

    /// <summary>Address the next DMC DMA reads.</summary>
    public ushort DmcDmaAddress => _dmc.FetchAddress;

    /// <summary>Hands the byte a DMC DMA fetched to the memory reader.</summary>
    public void SupplyDmcByte(byte value) => _dmc.SupplyByte(value);

    /// <summary>Mono samples waiting to be read.</summary>
    public int AvailableSamples
    {
        get
        {
            lock (_bufferLock)
            {
                return _available;
            }
        }
    }

    /// <summary>Capacity of the output buffer, in mono samples.</summary>
    public int BufferCapacity
    {
        get
        {
            lock (_bufferLock)
            {
                return _buffer.Length;
            }
        }
    }

    /// <summary>Output sample rate in Hz (REQ-CORE-070).</summary>
    public int SampleRate
    {
        get => _sampleRate;
        set
        {
            ArgumentOutOfRangeException.ThrowIfLessThan(value, MinSampleRate);
            ArgumentOutOfRangeException.ThrowIfGreaterThan(value, MaxSampleRate);
            _sampleRate = value;

            // At least the 10 000 samples REQ-CORE-071 demands, and otherwise 200 ms, which
            // is the upper end of the range SPEC-11 REQ-AUD-005 recommends.
            int capacity = Math.Max(MinBufferSamples, value / 5);
            lock (_bufferLock)
            {
                if (capacity != _buffer.Length)
                {
                    _buffer = new float[capacity];
                    _readIndex = 0;
                    _writeIndex = 0;
                    _available = 0;
                }
            }

            RecomputeRatio();
        }
    }

    /// <summary>Speed factor: only the audio pacing changes, never the emulation (REQ-CORE-072).</summary>
    public double Speed
    {
        get => _speed;
        set
        {
            ArgumentOutOfRangeException.ThrowIfLessThanOrEqual(value, 0.0);
            _speed = value;
            RecomputeRatio();
        }
    }

    /// <summary>
    /// Fine adjustment of the resampling ratio a frontend may request, at most ±0.5 %
    /// (REQ-CORE-070, SPEC-11 REQ-AUD-005). It never alters the emulation.
    /// </summary>
    public double RatioAdjustment
    {
        get => _ratioAdjustment;
        set
        {
            if (Math.Abs(value) > MaxRatioAdjustment)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(value), value, $"The fine adjustment is capped at ±{MaxRatioAdjustment:P1}.");
            }

            _ratioAdjustment = value;
            RecomputeRatio();
        }
    }

    /// <summary>
    /// Timings in force: they set the frame sequencer periods and select the noise and DMC
    /// period tables (REQ-CORE-015).
    /// </summary>
    public RegionTiming Timing
    {
        get => _timing;
        set
        {
            ArgumentNullException.ThrowIfNull(value);
            _timing = value;
            ApplyTiming();
            RecomputeRatio();
        }
    }

    /// <summary>Returns the machine to its power-on state, silent (REQ-CORE-004).</summary>
    /// <remarks>
    /// The frame sequencer is left where a $4017 write of $00, ten CPU cycles before the
    /// first instruction, would have put it. The CPU's own reset sequence takes seven cycles
    /// and the sequencer reset lands three cycles after the write, so starting the counter at
    /// zero reproduces the 9 to 12 cycle delay the hardware shows (blargg,
    /// <c>09.reset_timing</c>).
    /// </remarks>
    public void Reset()
    {
        _pulse1.Reset();
        _pulse2.Reset();
        _triangle.Reset();
        _noise.Reset();
        _dmc.Reset();
        ApplyTiming();

        _cycles = 0;
        _apuCycle = false;
        _frameCounter = 0;
        _frameStep = 0;
        _fiveStepMode = false;
        _irqInhibited = false;
        _frameIrqFlag = false;
        _frameIrqPending = false;
        _pendingSequencerReset = 0;
        _readableStatus = 0;
        _levelKey = -1;
        _level = 0.0;

        DiscardSamples();
    }

    /// <summary>
    /// Drops every pending sample and the resampler's history, so that the audio restarts
    /// from silence (REQ-CORE-074). Reset and closing the ROM do this.
    /// </summary>
    public void DiscardSamples()
    {
        _resampler.Reset();
        DropPendingSamples();
    }

    /// <summary>
    /// Empties the queue of samples the host has not read yet, leaving the resampler's own
    /// history alone (REQ-CORE-074; Q-006).
    /// </summary>
    /// <remarks>
    /// This is what restoring a savestate does. The pending samples belong to a future that
    /// no longer happens, so they go; the resampler's filter history is part of the APU state
    /// the block carries (REQ-CORE-090) and is restored with it, so that a rollback picks the
    /// sound up where the snapshot left it instead of cutting it (SPEC-11 REQ-AUD-012).
    /// </remarks>
    public void DropPendingSamples()
    {
        lock (_bufferLock)
        {
            _readIndex = 0;
            _writeIndex = 0;
            _available = 0;
        }
    }

    /// <summary>Advances the APU by one CPU cycle.</summary>
    public void StepCpuCycle()
    {
        // The frame counter's "set the IRQ flag" signal reaches the flag at the end of the
        // cycle it is raised on, so the CPU can only see it from the next cycle. blargg's
        // 08.irq_timing measures exactly that: the handler must start one instruction later
        // than an immediate set would give.
        if (_frameIrqPending)
        {
            _frameIrqPending = false;
            _frameIrqFlag = true;
        }

        // A CPU read of $4015 sees the register as it stood before this cycle's internal
        // clocking, the same one-clock lag that makes a register write land after it. The
        // bus performs the access after stepping the machine, so the value a read may return
        // is latched here, right after the flag above.
        _readableStatus = ComposeStatus(0);

        _cycles++;

        if (_pendingSequencerReset > 0 && --_pendingSequencerReset == 0)
        {
            _frameCounter = 0;
            _frameStep = 0;
        }

        StepFrameCounter();

        _apuCycle = !_apuCycle;
        if (_apuCycle)
        {
            // The pulse timers run at half the CPU clock (nesdev).
            _pulse1.ClockTimer();
            _pulse2.ClockTimer();
        }

        _triangle.ClockTimer();
        _noise.ClockTimer();
        _dmc.ClockTimer();

        _resampler.StepCycle(CurrentLevel());
    }

    // ---------------------------------------------------------------- frame counter

    private void StepFrameCounter()
    {
        int[] sequence = Sequence();
        _frameCounter++;

        if (_frameCounter != sequence[_frameStep])
        {
            return;
        }

        if (_fiveStepMode)
        {
            // Mode 1: quarter frames on steps 0 to 2, half frames on steps 1 and 4, and no
            // IRQ at all (nesdev).
            switch (_frameStep)
            {
                case 0:
                case 2:
                    ClockQuarterFrame();
                    break;
                case 1:
                case 4:
                    ClockQuarterFrame();
                    ClockHalfFrame();
                    break;
                default:
                    break;
            }
        }
        else
        {
            // Mode 0: the IRQ flag is set on the last three cycles of the sequence, and the
            // final quarter and half frames fall on the middle one (nesdev; blargg's
            // 07.irq_flag_timing measures all three).
            switch (_frameStep)
            {
                case 0:
                case 2:
                    ClockQuarterFrame();
                    break;
                case 1:
                    ClockQuarterFrame();
                    ClockHalfFrame();
                    break;
                case 4:
                    ClockQuarterFrame();
                    ClockHalfFrame();
                    SetFrameIrq();
                    break;
                default:
                    SetFrameIrq();
                    break;
            }
        }

        _frameStep++;
        if (_frameStep == sequence.Length)
        {
            _frameStep = 0;
            _frameCounter = 0;
        }
    }

    private int[] Sequence() => _timing.UsesPalNoiseAndDmcPeriods
        ? _fiveStepMode ? PalFiveStep : PalFourStep
        : _fiveStepMode ? NtscFiveStep : NtscFourStep;

    private void SetFrameIrq()
    {
        if (!_irqInhibited)
        {
            _frameIrqPending = true;
        }
    }

    private void ClockQuarterFrame()
    {
        _pulse1.Envelope.Clock();
        _pulse2.Envelope.Clock();
        _noise.Envelope.Clock();
        _triangle.ClockLinearCounter();
    }

    private void ClockHalfFrame()
    {
        _pulse1.Length.Clock(_cycles);
        _pulse2.Length.Clock(_cycles);
        _triangle.Length.Clock(_cycles);
        _noise.Length.Clock(_cycles);

        _pulse1.ClockSweep();
        _pulse2.ClockSweep();
    }

    // ---------------------------------------------------------------- registers

    /// <summary>
    /// Reads $4015, the status register: length counters in bits 0 to 3, DMC activity in
    /// bit 4, open bus in bit 5, frame IRQ in bit 6 and DMC IRQ in bit 7 (nesdev).
    /// </summary>
    /// <param name="openBus">Last value on the CPU data bus, which drives bit 5.</param>
    /// <remarks>
    /// <para>
    /// The value returned is the one latched at the start of the current CPU cycle, before
    /// this cycle's frame-counter events: a read on the very cycle a length counter is
    /// clocked, or the frame IRQ flag is set, still sees the old value. blargg's
    /// <c>04.clock_jitter</c>, <c>05.len_timing_mode0</c> and <c>07.irq_flag_timing</c> all
    /// turn on that one-cycle lag.
    /// </para>
    /// <para>
    /// The read clears the frame IRQ flag but leaves the DMC's alone (apu_test
    /// <c>7-dmc_basics</c>: "Reading IRQ flag shouldn't clear it"). The flag is re-raised on
    /// the next two cycles of the sequence, which is why blargg advises polling it twice.
    /// </para>
    /// </remarks>
    public byte ReadStatus(byte openBus = 0)
    {
        byte status = (byte)(_readableStatus | (openBus & 0x20));
        _frameIrqFlag = false;
        return status;
    }

    /// <summary>Reads $4015 with no side effect at all (REQ-CORE-081).</summary>
    public byte PeekStatus() => ComposeStatus(0);

    private byte ComposeStatus(byte openBus)
    {
        byte status = (byte)(openBus & 0x20);

        if (_pulse1.Length.Active)
        {
            status |= 0x01;
        }

        if (_pulse2.Length.Active)
        {
            status |= 0x02;
        }

        if (_triangle.Length.Active)
        {
            status |= 0x04;
        }

        if (_noise.Length.Active)
        {
            status |= 0x08;
        }

        if (_dmc.Active)
        {
            status |= 0x10;
        }

        if (_frameIrqFlag)
        {
            status |= 0x40;
        }

        if (_dmc.IrqFlag)
        {
            status |= 0x80;
        }

        return status;
    }

    /// <summary>Writes an APU register, <paramref name="address"/> in $4000-$4017.</summary>
    public void Write(ushort address, byte value)
    {
        switch (address)
        {
            case >= 0x4000 and <= 0x4003:
                _pulse1.Write(address - 0x4000, value, _cycles);
                break;

            case >= 0x4004 and <= 0x4007:
                _pulse2.Write(address - 0x4004, value, _cycles);
                break;

            case >= 0x4008 and <= 0x400B:
                _triangle.Write(address - 0x4008, value, _cycles);
                break;

            case >= 0x400C and <= 0x400F:
                _noise.Write(address - 0x400C, value, _cycles);
                break;

            case >= 0x4010 and <= 0x4013:
                _dmc.Write(address - 0x4010, value);
                break;

            case 0x4015:
                _pulse1.Length.SetEnabled((value & 0x01) != 0);
                _pulse2.Length.SetEnabled((value & 0x02) != 0);
                _triangle.Length.SetEnabled((value & 0x04) != 0);
                _noise.Length.SetEnabled((value & 0x08) != 0);
                _dmc.SetEnabled((value & 0x10) != 0);
                break;

            case 0x4017:
                WriteFrameCounter(value);
                break;

            default:
                break;
        }
    }

    private void WriteFrameCounter(byte value)
    {
        _fiveStepMode = (value & 0x80) != 0;
        _irqInhibited = (value & 0x40) != 0;

        if (_irqInhibited)
        {
            // The flag is cleared only by a $4015 read or by a $4017 write with bit 6 set
            // (blargg, "Misc").
            _frameIrqFlag = false;
            _frameIrqPending = false;
        }

        // The divider and the sequencer are reset three CPU cycles after the write, four when
        // it lands between APU cycles (nesdev; blargg's 04.clock_jitter measures both). The
        // write itself is handled after the cycle has been stepped, so the count below starts
        // from the cycle that follows it.
        _pendingSequencerReset = _apuCycle ? 3 : 4;

        if (_fiveStepMode)
        {
            // Mode 1 generates a quarter and a half frame signal straight away, without
            // waiting for the sequencer reset (blargg's mode 1 timing table puts it one cycle
            // after the write).
            ClockQuarterFrame();
            ClockHalfFrame();
        }
    }

    // ---------------------------------------------------------------- output

    private void ApplyTiming()
    {
        _noise.SetTiming(_timing);
        _dmc.SetTiming(_timing);
    }

    private void RecomputeRatio()
    {
        double cyclesPerFrame =
            (double)RegionTiming.DotsPerScanline * _timing.ScanlinesPerFrame
            * _timing.DotsPerCycleDenominator / _timing.DotsPerCycleNumerator;

        // REQ-CORE-070: a frame at speed 1 yields sampleRate / nominalFps samples, which is
        // 800 at 48 kHz in NTSC and 960 in PAL (DEC-R2-T3). REQ-CORE-072 divides that by the
        // speed factor; the frontend's fine adjustment rides on top.
        double samplesPerFrame = _sampleRate / (_timing.NominalFramesPerSecond * _speed);
        double samplesPerCycle = samplesPerFrame * (1.0 + _ratioAdjustment) / cyclesPerFrame;

        _resampler.SetRatio(samplesPerCycle, _sampleRate);
    }

    /// <summary>
    /// The mixed output level of the five channels, clamped to [−1, 1] (REQ-CORE-073).
    /// </summary>
    /// <remarks>
    /// The five channel outputs are small integers, so the mix is only recomputed when one of
    /// them actually moves; most CPU cycles change nothing.
    /// </remarks>
    private double CurrentLevel()
    {
        int pulse1 = _pulse1.Output;
        int pulse2 = _pulse2.Output;
        int triangle = _triangle.Output;
        int noise = _noise.Output;
        int dmc = _dmc.Output;

        int key = pulse1 | (pulse2 << 4) | (triangle << 8) | (noise << 12) | (dmc << 16);
        if (key == _levelKey)
        {
            return _level;
        }

        _levelKey = key;

        // The linear approximation of nesdev's "APU Mixer", which REQ-CORE-073 prescribes.
        double mix =
            (PulseStep * (pulse1 + pulse2))
            + (TriangleStep * triangle)
            + (NoiseStep * noise)
            + (DmcStep * dmc);

        _level = Math.Clamp(OutputGain * mix, -1.0, 1.0);
        return _level;
    }

    private void Enqueue(float sample)
    {
        lock (_bufferLock)
        {
            if (_available == _buffer.Length)
            {
                // On overflow the oldest samples are dropped (REQ-CORE-071,
                // SPEC-11 REQ-AUD-005).
                _readIndex = (_readIndex + 1) % _buffer.Length;
                _available--;
            }

            _buffer[_writeIndex] = sample;
            _writeIndex = (_writeIndex + 1) % _buffer.Length;
            _available++;
        }
    }

    /// <summary>
    /// Fills <paramref name="destination"/> with audio (REQ-CORE-071).
    /// </summary>
    /// <param name="destination">Buffer to fill entirely, or not at all.</param>
    /// <param name="stereo">
    /// When <see langword="true"/> the buffer is interleaved L, R, L, R with L = R, and
    /// half as many mono samples are consumed.
    /// </param>
    /// <returns>
    /// <paramref name="destination"/>.Length when the buffer was filled, 0 when there were
    /// not enough samples, in which case nothing is consumed and the host pads with
    /// silence.
    /// </returns>
    /// <remarks>May be called from a thread other than the one running frames.</remarks>
    public int ReadSamples(Span<float> destination, bool stereo)
    {
        if (destination.IsEmpty)
        {
            return 0;
        }

        if (stereo && (destination.Length % 2) != 0)
        {
            throw new ArgumentException("A stereo buffer must hold an even number of values.", nameof(destination));
        }

        int needed = stereo ? destination.Length / 2 : destination.Length;

        lock (_bufferLock)
        {
            if (_available < needed)
            {
                return 0;
            }

            for (int i = 0; i < needed; i++)
            {
                float sample = _buffer[_readIndex];
                _readIndex = (_readIndex + 1) % _buffer.Length;

                if (stereo)
                {
                    destination[2 * i] = sample;
                    destination[(2 * i) + 1] = sample;
                }
                else
                {
                    destination[i] = sample;
                }
            }

            _available -= needed;
        }

        return destination.Length;
    }

    // ---------------------------------------------------------------- savestate

    /// <summary>Bytes the APU's part of a savestate occupies at most (REQ-ARCH-013).</summary>
    public static int MaxStateSize => 512 + BandLimitedResampler.StateSize;

    /// <summary>Appends the APU state to <paramref name="writer"/> (REQ-CORE-090).</summary>
    public void SaveState(StateWriter writer)
    {
        ArgumentNullException.ThrowIfNull(writer);

        _pulse1.SaveState(writer);
        _pulse2.SaveState(writer);
        _triangle.SaveState(writer);
        _noise.SaveState(writer);
        _dmc.SaveState(writer);

        writer.WriteInt64(_cycles);
        writer.WriteBool(_apuCycle);
        writer.WriteInt32(_frameCounter);
        writer.WriteInt32(_frameStep);
        writer.WriteBool(_fiveStepMode);
        writer.WriteBool(_irqInhibited);
        writer.WriteBool(_frameIrqFlag);
        writer.WriteBool(_frameIrqPending);
        writer.WriteInt32(_pendingSequencerReset);
        writer.WriteByte(_readableStatus);
        writer.WriteInt32(_levelKey);
        writer.WriteDouble(_level);

        _resampler.SaveState(writer);
    }

    /// <summary>Restores the APU state from <paramref name="reader"/> (REQ-CORE-091).</summary>
    public void LoadState(StateReader reader)
    {
        ArgumentNullException.ThrowIfNull(reader);

        _pulse1.LoadState(reader);
        _pulse2.LoadState(reader);
        _triangle.LoadState(reader);
        _noise.LoadState(reader);
        _dmc.LoadState(reader);

        _cycles = reader.ReadInt64();
        _apuCycle = reader.ReadBool();
        _frameCounter = reader.ReadInt32();
        _frameStep = reader.ReadInt32();
        _fiveStepMode = reader.ReadBool();
        _irqInhibited = reader.ReadBool();
        _frameIrqFlag = reader.ReadBool();
        _frameIrqPending = reader.ReadBool();
        _pendingSequencerReset = reader.ReadInt32();
        _readableStatus = reader.ReadByte();
        _levelKey = reader.ReadInt32();
        _level = reader.ReadDouble();

        _resampler.LoadState(reader);

        // Pending audio is dropped on restore (REQ-CORE-074); the resampler's own history was
        // restored above, so the sound picks up from the state that was saved (Q-006).
        DropPendingSamples();
    }
}
