//! The variant mechanism of the WGSL sources: what `#ifdef` is to the GLSL ones.
//!
//! The OpenGL backend compiles one program per combination of six independent axes
//! (`gl/shaders.rs`: `solid × srgb_palette × bend × centroid × voxel_grid × pixel_shadow`, plus
//! the floor mask), and **the absence of a define is the point** — at curvature 0, or with the
//! voxel grid off, the compiler is handed the program of before the amendment, with no uniform
//! tested per vertex and no varying for what the variant would have produced. That is how the
//! Classic frame pays nothing for the Enhanced style, and RU-7 has to reproduce it on this side
//! rather than fold the axes into run-time branches.
//!
//! WGSL has no preprocessor, so this is it: the markers are **WGSL line comments**, which keeps
//! every source a valid WGSL file that an editor, a formatter and `naga` can still read whole.
//!
//! ```text
//! //#ifdef ZAX_BEND
//! //#ifndef ZAX_SOLID
//! //#if ZAX_VOXEL_GRID || ZAX_PIXEL_SHADOW
//! //#else
//! //#endif
//! ```
//!
//! A condition is a name, or names joined by `||` — the three forms the GLSL sources use between
//! them (`#ifdef`, `#ifndef`, `#if defined(A) || defined(B)`). Conditions nest. Anything else is
//! a mistake in a source of this crate rather than input, so it is reported by
//! [`Expansion::problem`] and the offending block is kept rather than guessed at: a WGSL source
//! that comes out wrong fails at module creation, loudly, which is where a shader fault belongs.
//!
//! What the expander does **not** do is substitute anything inside a line. Everything the two
//! APIs disagree about — the 0..1 clip depth, the top-left texture origin, the binding model —
//! stays written out by hand in the WGSL, as it was before this module existed.

/// How a line of markers was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    /// `//#ifdef A`, `//#ifndef A` or `//#if A || B`, with the condition already evaluated.
    If(bool),
    Else,
    Endif,
    /// A marker line that is not one of the three above.
    Bad,
}

/// The result of expanding a source: the WGSL, and the first malformed marker if there was one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expansion {
    /// The expanded WGSL.
    pub wgsl: String,
    /// The first marker line that could not be read, if any.
    pub problem: Option<String>,
}

/// Reads one marker line, `rest` being what follows `//#`.
fn marker(rest: &str, defines: &[&str]) -> Marker {
    let has = |name: &str| !name.is_empty() && defines.contains(&name);
    let (word, argument) = match rest.find(char::is_whitespace) {
        Some(at) => (&rest[..at], rest[at..].trim()),
        None => (rest, ""),
    };
    match word {
        "else" if argument.is_empty() => Marker::Else,
        "endif" if argument.is_empty() => Marker::Endif,
        "ifdef" if !argument.is_empty() && !argument.contains(char::is_whitespace) => {
            Marker::If(has(argument))
        }
        "ifndef" if !argument.is_empty() && !argument.contains(char::is_whitespace) => {
            Marker::If(!has(argument))
        }
        // A disjunction of names: `A`, or `A || B || ...`. Every term has to be a bare name.
        "if" if !argument.is_empty() => {
            let mut any = false;
            for term in argument.split("||") {
                let name = term.trim();
                if name.is_empty() || name.contains(char::is_whitespace) {
                    return Marker::Bad;
                }
                any |= has(name);
            }
            Marker::If(any)
        }
        _ => Marker::Bad,
    }
}

/// `source` with the marker blocks `defines` does not select removed, the marker lines with them.
///
/// The lines that survive keep their text and their order exactly, so a line number of the
/// expansion maps onto the source by counting the lines the expansion dropped — which is what
/// makes a `naga` error message about the expansion readable against the file.
#[must_use]
pub fn expand(source: &str, defines: &[&str]) -> Expansion {
    // One entry per open condition: whether its lines are being kept, and whether any branch of
    // it has been taken yet (so that `else` knows what to do inside a dropped outer block).
    let mut stack: Vec<(bool, bool)> = Vec::new();
    let mut problem = None;
    let mut wgsl = String::with_capacity(source.len());
    let keeping = |stack: &[(bool, bool)]| stack.iter().all(|&(keep, _)| keep);

    for line in source.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("//#") else {
            if keeping(&stack) {
                wgsl.push_str(line);
                wgsl.push('\n');
            }
            continue;
        };
        match marker(rest, defines) {
            Marker::If(taken) => stack.push((taken, taken)),
            Marker::Else => match stack.last_mut() {
                Some((keep, taken)) => {
                    *keep = !*taken;
                    *taken = true;
                }
                None => problem = problem.or_else(|| Some(line.trim().to_owned())),
            },
            Marker::Endif => {
                if stack.pop().is_none() {
                    problem = problem.or_else(|| Some(line.trim().to_owned()));
                }
            }
            Marker::Bad => {
                // Kept, not guessed at: the line stays in the WGSL as the comment it is, and the
                // problem is reported. A condition that was meant to drop code therefore shows
                // up as code that does not compile, not as code that silently runs.
                problem = problem.or_else(|| Some(line.trim().to_owned()));
                if keeping(&stack) {
                    wgsl.push_str(line);
                    wgsl.push('\n');
                }
            }
        }
    }
    if !stack.is_empty() {
        problem = problem.or_else(|| Some(format!("{} unterminated //#if", stack.len())));
    }
    Expansion { wgsl, problem }
}

/// The chunks of one module, expanded with `defines` and joined in order.
#[must_use]
pub fn compose(parts: &[&str], defines: &[&str]) -> Expansion {
    let mut wgsl = String::new();
    let mut problem = None;
    for part in parts {
        let expansion = expand(part, defines);
        wgsl.push_str(&expansion.wgsl);
        wgsl.push('\n');
        problem = problem.or(expansion.problem);
    }
    Expansion { wgsl, problem }
}

#[cfg(test)]
mod tests {
    use super::{compose, expand};

    #[test]
    fn a_source_without_markers_is_handed_back_line_for_line() {
        let source = "fn f() {\n    return;\n}\n";
        let expanded = expand(source, &["ZAX_BEND"]);
        assert_eq!(expanded.wgsl, source);
        assert_eq!(expanded.problem, None);
    }

    #[test]
    fn ifdef_keeps_its_block_only_when_the_name_is_defined() {
        let source = "a\n//#ifdef ZAX_BEND\nb\n//#endif\nc\n";
        assert_eq!(expand(source, &[]).wgsl, "a\nc\n");
        assert_eq!(expand(source, &["ZAX_BEND"]).wgsl, "a\nb\nc\n");
    }

    #[test]
    fn ifndef_is_the_other_way_round() {
        let source = "//#ifndef ZAX_SOLID\ndiscard;\n//#endif\n";
        assert_eq!(expand(source, &[]).wgsl, "discard;\n");
        assert_eq!(expand(source, &["ZAX_SOLID"]).wgsl, "");
    }

    #[test]
    fn else_takes_the_branch_the_condition_did_not() {
        let source = "//#ifdef ZAX_BEND\nbent\n//#else\nflat\n//#endif\n";
        assert_eq!(expand(source, &["ZAX_BEND"]).wgsl, "bent\n");
        assert_eq!(expand(source, &[]).wgsl, "flat\n");
    }

    #[test]
    fn a_disjunction_holds_when_any_of_its_names_is_defined() {
        let source = "//#if ZAX_VOXEL_GRID || ZAX_PIXEL_SHADOW\nlocal\n//#endif\n";
        assert_eq!(expand(source, &[]).wgsl, "");
        assert_eq!(expand(source, &["ZAX_VOXEL_GRID"]).wgsl, "local\n");
        assert_eq!(expand(source, &["ZAX_PIXEL_SHADOW"]).wgsl, "local\n");
        assert_eq!(
            expand(source, &["ZAX_VOXEL_GRID", "ZAX_PIXEL_SHADOW"]).wgsl,
            "local\n"
        );
    }

    #[test]
    fn conditions_nest_and_an_outer_miss_drops_the_whole_block() {
        let source = "//#ifdef A\nouter\n//#ifdef B\ninner\n//#else\nnot_b\n//#endif\n//#endif\n";
        assert_eq!(expand(source, &[]).wgsl, "");
        assert_eq!(expand(source, &["A"]).wgsl, "outer\nnot_b\n");
        assert_eq!(expand(source, &["A", "B"]).wgsl, "outer\ninner\n");
        // The inner `else` must not resurrect a block its outer condition dropped.
        assert_eq!(expand(source, &["B"]).wgsl, "");
    }

    #[test]
    fn an_indented_marker_is_still_a_marker() {
        let source = "    //#ifdef A\n    kept\n    //#endif\n";
        assert_eq!(expand(source, &["A"]).wgsl, "    kept\n");
        assert_eq!(expand(source, &[]).wgsl, "");
    }

    #[test]
    fn a_plain_comment_is_not_a_marker() {
        let source = "// #ifdef A\n//## not a marker\nkept\n";
        let expanded = expand(source, &[]);
        // `// #ifdef A` has a space after the slashes, so it is an ordinary comment and passes
        // through untouched. `//##` does parse as a marker line, of the unreadable word "#": it
        // is reported, and its line is kept as the comment it is.
        assert_eq!(expanded.wgsl, "// #ifdef A\n//## not a marker\nkept\n");
        assert_eq!(expanded.problem.as_deref(), Some("//## not a marker"));
    }

    #[test]
    fn a_malformed_marker_is_reported_and_its_block_is_kept() {
        let expanded = expand("//#ifdef\nbody\n//#endif\n", &[]);
        assert_eq!(expanded.problem.as_deref(), Some("//#ifdef"));
        assert_eq!(expanded.wgsl, "//#ifdef\nbody\n");
        let stray = expand("body\n//#endif\n", &[]);
        assert_eq!(stray.problem.as_deref(), Some("//#endif"));
    }

    #[test]
    fn an_unterminated_condition_is_reported() {
        let expanded = expand("//#ifdef A\nbody\n", &["A"]);
        assert_eq!(expanded.problem.as_deref(), Some("1 unterminated //#if"));
    }

    #[test]
    fn compose_joins_the_chunks_in_order_and_keeps_the_first_problem() {
        let composed = compose(&["a\n", "//#ifdef A\nb\n//#endif\n", "c\n"], &["A"]);
        assert_eq!(composed.wgsl, "a\n\nb\n\nc\n\n");
        assert_eq!(composed.problem, None);
        let broken = compose(&["//#endif\n", "//#else\n"], &[]);
        assert_eq!(broken.problem.as_deref(), Some("//#endif"));
    }
}
