# tests/Acceptance

**Role R4, round-1 verification** (PLAN.md, phase P4). Written from the specification alone, by
a session that is not the implementers'.

Rules:

- The only sources are `cleanroom/spec/`, `cleanroom/assets-neutral/`, `cleanroom/impl/` (read
  for inspection, never modified under `src/`) and public documentation.
- Every test carries the `REQ-<MOD>-NNN` identifier it verifies in a comment; the verdicts go
  into `cleanroom/verify/round1/traceability.csv` and `R1-report-part-a.md`.
- Implementers do not add tests here. Implementer tests live in `tests/Zax.<Module>.Tests`.
- `Zax.Harness` (in `tools/`) is the headless driver used for the exports and the offscreen
  captures of REQ-ARCH-031; its outputs are archived in `verify/round1/part-a/`.
- A test that demonstrates a NON CONFORME verdict is marked `[Trait("Verdict", "NonConforme")]`
  and is meant to be red until the requirement is met.

## Part A (SPEC-01, 02, 03, 04, 04B, 12)

| File | Covers |
|---|---|
| `SpecRom.cs` | The minimal test ROM of SPEC-02 §5.1 and its set-up procedure |
| `CoreInterfaceTests.cs` | SPEC-02 §3.1 to §3.4: lifecycle, ROM loading, regions, frames, input word |
| `CoreTileContractTests.cs` | SPEC-02 §3.5 and the vectors V1 to V10 of §5.3 |
| `CoreMmc2Tests.cs` | REQ-CORE-039 and REQ-CORE-044 on a synthetic mapper 9 ROM |
| `CoreZapperAudioTests.cs` | SPEC-02 §3.6 (Zapper) and §3.7 (audio) |
| `CoreResamplerSpectrumTests.cs` | The band-limiting half of REQ-CORE-070 (FFT) |
| `CoreMemoryStateTests.cs` | SPEC-02 §3.8 to §3.11 and §3.13 |
| `RomSuite.cs`, `CoreAccuracyTests.cs` | SPEC-02 §3.12 and REQ-ACC-001, public test ROMs |
| `ProfileWireScanner.cs`, `ProfileFormatTests.cs` | SPEC-03 in full, with an independent protobuf wire reader |
| `ConversionSupport.cs`, `ConversionTests.cs` | SPEC-04 and the vectors TV-E3D-01 to TV-E3D-14 |
| `GeometryTests.cs` | SPEC-04B and the vectors TV-G, TV-C, TV-Y, TV-M, TV-W, TV-K |
| `ScriptHookTests.cs` | The Lua hook points of SPEC-04 §3.14 and REQ-E3D-455 |
| `RemainingRulesTests.cs` | 2D mode, deformation, direct texture, clipping, catalogue, user profiles |
| `RomAndRemainderTests.cs` | The "auto" halves of the [ROM] criteria, and the user's own ROMs |
| `AcceptanceGateTests.cs` | SPEC-12 and the performance budgets |
| `ArchitectureTests.cs` | SPEC-01 |
| `NeutralAssets.cs` | Locates `assets-neutral` from the test binary |

## Part B (SPEC-05, 06, 07, 11)

| File | Covers |
|---|---|
| `LuaSupport.cs` | Profiles with scripts and a Lua runtime started in the load order of REQ-LUA-015 |
| `SpecTables.cs` | Reads the member tables of SPEC-05 §4 out of the specification document itself |
| `HostDriver.cs` | Drives the desktop emulation loop headless; `EmulationHost` and `GameSession` are internal to `Zax.App`, so they are reached by reflection over the shipped assembly rather than by changing anything under `src/` |
| `SceneSupport.cs` | Synthetic scenes, the hidden GL 3.3 core target and the PNG writer for the archived captures |
| `LuaModelTests.cs` | SPEC-05 §3.1 to §3.3: script model, order, linking, Lua state, life cycle |
| `LuaFrameTests.cs` | SPEC-05 §3.4 and §3.5: frame hooks, `BuildPattern`, `DetectShape`, `InputProcess` |
| `LuaBindingTests.cs` | SPEC-05 §3.6 to §3.8: value binding, libraries, errors |
| `LuaApiTests.cs` | SPEC-05 §3.9 to §3.12: persistent data, the whole §4 API, the dialect |
| `LuaHostTests.cs` | What only the host shows: load order, end-of-game resets, rollback, the whole corpus run dynamically |
| `SceneCameraTests.cs` | SPEC-06 §3.1 to §3.8 and the vectors of §5.1 |
| `SceneRenderTests.cs` | SPEC-06 §3.9 to §3.17 without a GL context, and the vectors of §5.2 and §5.3 |
| `SceneCaptureTests.cs` | SPEC-06 offscreen captures; archived in `verify/round1/part-b/` |
| `LoopTests.cs` | SPEC-07 in full, and the vectors T1 to T10 |
| `AudioTests.cs` | SPEC-11 in full, and the vectors A1 to A7 |
