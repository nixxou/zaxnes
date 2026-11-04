//! The Markdown report of a campaign: one table per suite, a summary, and for every `fail` or
//! `timeout` a line saying which subsystem it belongs to.
//!
//! ## The reference column
//!
//! `nes-test-roms/status.txt` ships inside the collection itself and is its own result table,
//! with a column for NESICIDE, one for Nestopia 1.40 and one for Nintendulator 0.975. It is
//! read straight from the ROM directory the campaign was pointed at, so the report quotes the
//! file the ROMs came with and invents nothing. A ROM the file does not list simply has no
//! reference.
//!
//! No published Mesen 2 table covers these ROMs row by row, so there is no Mesen 2 column: an
//! empty column is better than a guessed one.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::{Outcome, RomResult, Summary};
use crate::HarnessError;

/// The file inside the collection that carries its own result table.
const STATUS_FILE: &str = "nes-test-roms/status.txt";

/// Where that file comes from, for the report to cite.
const STATUS_SOURCE: &str =
    "https://github.com/christopherpow/nes-test-roms/blob/master/status.txt";

/// Writes the whole report.
///
/// # Errors
///
/// [`HarnessError::Text`] if the Markdown cannot be formatted.
pub fn markdown(
    roms_dir: &Path,
    results: &[RomResult],
    summary: Summary,
) -> Result<String, HarnessError> {
    let reference = Reference::read(roms_dir);
    let mut out = String::new();

    writeln!(out, "# MF-T — Zaxnes against the public test ROMs\n")?;
    writeln!(
        out,
        "Produced by `zax-harness test-roms <roms dir> <out.json> --markdown <this file>`. The \
         ROMs are **not** in the repository: they come from `assets-free/test-roms/`, whose \
         `README.md` records where each suite was downloaded from and at which commit. The JSON \
         and the per-ROM PNGs of the final frame stay under `rust/target-mf/`.\n"
    )?;
    writeln!(
        out,
        "Each suite is read the way its own documentation says, never by guessing:\n\n\
         * **blargg's `$6000` protocol** (`nes-test-roms/*/readme.txt`, \"Output at $6000\"): \
         the `DE B0 61` signature at `$6001-$6003`, `$80` while running, `$81` as a request for \
         the RESET button at least 100 ms later, anything else a result code with 0 for a pass, \
         and the zero-terminated text from `$6004`. A ROM that prints \"Done\" rather than \
         \"Passed\" exits with code 0 without having passed — the readme says those have to be \
         watched or heard — so it is `manual`.\n\
         * **the screen**, for the suites that predate that protocol. \
         `sprite_hit_tests_2005.10.05` and `sprite_overflow_tests` draw `PASSED` or `FAILED #n`; \
         `blargg_ppu_tests_2005.09.15b` draws a hexadecimal code whose readme says \"A result \
         code of 1 always indicates that all tests were passed\".\n\
         * **Holy Mapperel's result screen** (`holy-mapperel/README.md`, \"Displayed result\"): \
         the detected board, the memory sizes and the four digits of `DETAILED TEST RESULT`, of \
         which the README says \"Zero is normal; anything else reflects something unexpected\".\n\
         * **`AccuracyCoin`'s result bytes** at `$0403-$0498`, one per test, plus its own pass \
         tally.\n\n\
         Anything none of those can read is `manual`, and the PNG of the final frame is the \
         evidence.\n"
    )?;

    write_totals(&mut out, summary)?;
    write_suite_summary(&mut out, results)?;
    write_diagnoses(&mut out, results)?;
    write_tables(&mut out, results, &reference)?;

    writeln!(
        out,
        "\n---\n\nThe reference columns are quoted from `{STATUS_FILE}`, the collection's own \
         result table, as it ships with the ROMs ([source]({STATUS_SOURCE})). A blank cell means \
         that file does not list the ROM. There is no Mesen 2 column: no published table covers \
         these ROMs row by row, and a guessed column would be worse than none."
    )?;

    Ok(out)
}

fn write_totals(out: &mut String, summary: Summary) -> Result<(), HarnessError> {
    writeln!(out, "## Totals\n")?;
    writeln!(out, "| Result | ROMs |")?;
    writeln!(out, "|---|---:|")?;
    writeln!(out, "| pass | {} |", summary.pass)?;
    writeln!(out, "| fail | {} |", summary.fail)?;
    writeln!(out, "| timeout | {} |", summary.timeout)?;
    writeln!(out, "| unsupported | {} |", summary.unsupported)?;
    writeln!(out, "| manual | {} |", summary.manual)?;
    writeln!(out, "| **total** | **{}** |\n", summary.total)?;
    Ok(())
}

fn write_suite_summary(out: &mut String, results: &[RomResult]) -> Result<(), HarnessError> {
    writeln!(out, "## Per suite\n")?;
    writeln!(
        out,
        "| Suite | passed / total | fail | timeout | unsupported | manual |"
    )?;
    writeln!(out, "|---|---:|---:|---:|---:|---:|")?;
    for (suite, rows) in group(results) {
        let mut counts = Summary::default();
        for row in &rows {
            counts.count(row.outcome);
        }
        writeln!(
            out,
            "| `{suite}` | {} / {} | {} | {} | {} | {} |",
            counts.pass,
            counts.total,
            counts.fail,
            counts.timeout,
            counts.unsupported,
            counts.manual
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_diagnoses(out: &mut String, results: &[RomResult]) -> Result<(), HarnessError> {
    let bad: Vec<&RomResult> = results
        .iter()
        .filter(|row| matches!(row.outcome, Outcome::Fail | Outcome::Timeout))
        .collect();

    writeln!(out, "## Every failure and timeout, with its subsystem\n")?;
    if bad.is_empty() {
        writeln!(out, "None.\n")?;
        return Ok(());
    }
    writeln!(
        out,
        "One line per row that is not a pass. The subsystem is what the ROM exercises, which is \
         where a fix would go; this lot diagnoses and does not fix.\n"
    )?;
    writeln!(out, "| ROM | Result | Subsystem | What it says |")?;
    writeln!(out, "|---|---|---|---|")?;
    for row in bad {
        writeln!(
            out,
            "| `{}` | {} | {} | {} |",
            row.rom,
            row.outcome.label(),
            diagnose(&row.rom),
            cell(&row.message)
        )?;
    }
    writeln!(out)?;
    Ok(())
}

fn write_tables(
    out: &mut String,
    results: &[RomResult],
    reference: &Reference,
) -> Result<(), HarnessError> {
    writeln!(out, "## Every ROM, by suite\n")?;
    for (suite, rows) in group(results) {
        let mut counts = Summary::default();
        for row in &rows {
            counts.count(row.outcome);
        }
        writeln!(out, "### `{suite}`\n")?;
        writeln!(
            out,
            "**{} passed of {}**{}\n",
            counts.pass,
            counts.total,
            extra(&counts)
        )?;
        writeln!(
            out,
            "| ROM | Result | What the ROM says | NESICIDE | Nestopia 1.40 | Nintendulator 0.975 |"
        )?;
        writeln!(out, "|---|---|---|---|---|---|")?;
        for row in rows {
            let known = reference.of(&row.rom);
            writeln!(
                out,
                "| `{}` | {} | {} | {} | {} | {} |",
                file_of(&row.rom),
                row.outcome.label(),
                cell(&row.message),
                known.map_or("", |entry| entry.nesicide.as_str()),
                known.map_or("", |entry| entry.nestopia.as_str()),
                known.map_or("", |entry| entry.nintendulator.as_str()),
            )?;
        }
        writeln!(out)?;
    }
    Ok(())
}

/// `, 3 fail, 1 timeout` — what is in a suite besides its passes.
fn extra(counts: &Summary) -> String {
    let mut parts = Vec::new();
    for (count, word) in [
        (counts.fail, "fail"),
        (counts.timeout, "timeout"),
        (counts.unsupported, "unsupported"),
        (counts.manual, "manual"),
    ] {
        if count > 0 {
            parts.push(format!("{count} {word}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

/// The ROMs of each suite, suites in path order.
fn group(results: &[RomResult]) -> Vec<(String, Vec<&RomResult>)> {
    let mut suites: BTreeMap<String, Vec<&RomResult>> = BTreeMap::new();
    for row in results {
        suites.entry(row.suite.clone()).or_default().push(row);
    }
    suites.into_iter().collect()
}

/// The file name of a relative path.
fn file_of(rom: &str) -> &str {
    rom.rsplit('/').next().unwrap_or(rom)
}

/// A message, safe to put in a Markdown table cell and short enough to read.
fn cell(message: &str) -> String {
    const LIMIT: usize = 400;

    let mut text: String = message
        .chars()
        .map(|character| match character {
            '|' => '/',
            '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect();
    if text.chars().count() > LIMIT {
        text = text.chars().take(LIMIT).collect::<String>() + "…";
    }
    text
}

/// The subsystem the ROM at `rom` exercises.
///
/// This is read off the suite the ROM belongs to — which is exactly what each suite's readme
/// says it tests — and not from the failure, so it says where to look and claims nothing about
/// the cause.
fn diagnose(rom: &str) -> &'static str {
    const RULES: [(&str, &str); 42] = [
        (
            "accuracycoin",
            "CPU, PPU and APU together — see the per-test list in the JSON",
        ),
        ("holy-mapperel", "the mapper named in the row"),
        ("apu_mixer", "APU mixing"),
        ("apu_reset", "APU, after the RESET button"),
        ("apu_test", "APU length counters, IRQ and DMC"),
        ("blargg_apu", "APU length counters, IRQ and timing"),
        ("blargg_nes_cpu_test5", "CPU instructions"),
        ("blargg_ppu_tests", "PPU registers, palette and VRAM"),
        ("branch_timing", "CPU branch timing"),
        ("cpu_dummy_reads", "CPU dummy reads"),
        ("cpu_dummy_writes", "CPU read-modify-write double writes"),
        ("cpu_exec_space", "CPU fetching from PPU and APU space"),
        ("cpu_interrupts_v2", "CPU interrupts and their timing"),
        ("cpu_reset", "CPU and the RESET button"),
        ("cpu_timing_test6", "CPU instruction timing"),
        ("dmc_dma_during_read4", "DMC DMA stealing a read"),
        ("dmc_tests", "APU DMC"),
        ("dpcmletterbox", "DMC DMA and PPU timing"),
        ("exram", "MMC5 ExRAM"),
        ("fdsirqtests", "Famicom Disk System IRQ"),
        ("full_palette", "PPU palette"),
        ("instr_misc", "CPU instruction edge cases"),
        ("instr_test", "CPU instructions"),
        ("instr_timing", "CPU instruction timing"),
        ("m22chrbankingtest", "VRC2/VRC4 CHR banking"),
        ("mmc1_a12", "MMC1 and PPU A12"),
        ("mmc3_irq", "MMC3 IRQ"),
        ("mmc3_test", "MMC3 scanline counter and IRQ"),
        ("mmc5test", "MMC5"),
        ("nes_instr_test", "CPU instructions"),
        ("nmi_sync", "PPU/CPU NMI synchronisation"),
        ("oam_read", "PPU OAM reads"),
        ("oam_stress", "PPU OAM under stress"),
        ("ppu_open_bus", "PPU open bus"),
        ("ppu_read_buffer", "PPU read buffer"),
        ("ppu_vbl_nmi", "PPU VBlank and NMI timing"),
        ("read_joy3", "controller reads and DMC DMA"),
        ("scanline", "PPU scanline timing"),
        ("sprdma_and_dmc_dma", "OAM DMA against DMC DMA"),
        ("sprite_hit", "PPU sprite 0 hit"),
        ("sprite_overflow", "PPU sprite overflow"),
        ("vbl_nmi_timing", "PPU VBlank and NMI timing"),
    ];

    let lower = rom.to_ascii_lowercase();
    RULES
        .iter()
        .find(|(needle, _)| lower.contains(needle))
        .map_or("not classified — see the suite's readme", |(_, what)| {
            *what
        })
}

/// What `status.txt` says, ROM by ROM.
#[derive(Debug, Clone, Default)]
struct Reference {
    rows: BTreeMap<String, Entry>,
}

/// One row of `status.txt`.
#[derive(Debug, Clone, Default)]
struct Entry {
    nesicide: String,
    nestopia: String,
    nintendulator: String,
}

impl Reference {
    /// Reads `status.txt` from the ROM directory, or nothing when it is not there.
    fn read(roms_dir: &Path) -> Self {
        let Ok(text) = fs::read_to_string(roms_dir.join(STATUS_FILE)) else {
            return Self::default();
        };

        let mut rows = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with('|') {
                continue;
            }
            // `| NESICIDE | Nestopia | Nintendulator | notes | path | other |`
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            let (Some(nesicide), Some(nestopia), Some(nintendulator), Some(path)) =
                (fields.get(1), fields.get(2), fields.get(3), fields.get(5))
            else {
                continue;
            };
            if path.is_empty() || !path.to_ascii_lowercase().ends_with(".nes") {
                continue;
            }

            rows.insert(
                format!("nes-test-roms/{}", path.replace('\\', "/")),
                Entry {
                    nesicide: clean(nesicide),
                    nestopia: clean(nestopia),
                    nintendulator: clean(nintendulator),
                },
            );
        }

        Self { rows }
    }

    fn of(&self, rom: &str) -> Option<&Entry> {
        self.rows.get(rom)
    }
}

/// A `status.txt` cell: `????` there means "not sure yet", which is no reference at all.
fn clean(cell: &str) -> String {
    let cell = cell.trim();
    if cell.chars().all(|character| character == '?') {
        String::new()
    } else {
        cell.to_owned()
    }
}
