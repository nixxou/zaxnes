//! AMD-047 REQ-PERS-087: the four clip margins accept any finite real, negative included, and
//! a script's write is kept as it was written.
//!
//! The margins reach the conversion through `ScriptSettings::set_clipping`, which writes the
//! settings model and lets the model's domain decide (`Domain::finite_real`). Nothing on this
//! path bounds the value any more, so the tests here are the amendment's first acceptance
//! criterion read literally, plus the mirror into `ConversionSettings` that every frame uses.

use super::support::{ScriptProfileBuilder, ScriptSession};

fn session() -> ScriptSession {
    ScriptProfileBuilder::new().load()
}

/// REQ-PERS-087, first criterion: `Setting.Clipping[1] = -80` reads back −80, and
/// `Clipping.x = -128` puts −128 in `Setting.Clipping[0]`.
#[test]
fn req_pers_087_a_script_writes_a_negative_margin_and_reads_it_back() {
    let mut session = session();
    session.evaluate("Setting.Clipping[1] = -80");
    assert_eq!(-80.0, session.number("return Setting.Clipping[1]"));

    session.evaluate("Clipping.x = -128");
    assert_eq!(-128.0, session.number("return Setting.Clipping[0]"));
    assert_eq!(-128.0, session.number("return Clipping[0]"));
}

/// REQ-PERS-087: the four margins of the "Punch-Out!!" top-down ring, written one by one and as
/// a vector, and read back unchanged through both names of each one.
#[test]
fn req_pers_087_the_four_margins_keep_what_a_script_writes() {
    let mut session = session();
    session.evaluate("Clipping.target = Color(-128, -80, -128, 0)");
    for (index, expected) in [(0, -128.0), (1, -80.0), (2, -128.0), (3, 0.0)] {
        assert_eq!(
            expected,
            session.number(&format!("return Setting.Clipping[{index}]")),
            "margin {index}"
        );
    }

    // The same four through the named components, which address the same settings.
    session.evaluate("Clipping.x = -1.5; Clipping.y = -2.5; Clipping.z = -3.5; Clipping.w = -4.5");
    assert_eq!(-1.5, session.number("return Clipping.x"));
    assert_eq!(-2.5, session.number("return Clipping.y"));
    assert_eq!(-3.5, session.number("return Clipping.z"));
    assert_eq!(-4.5, session.number("return Clipping.w"));
}

/// REQ-PERS-087: a value that is not finite is out of domain and the margin takes its own
/// default (the general rule of REQ-PERS-003).
#[test]
fn req_pers_087_a_margin_a_script_writes_not_finite_takes_its_default() {
    // The defaults are `f32` and Lua reads them as `f64`, so they are widened rather than
    // written out: 0.1_f32 is not 0.1_f64.
    let mut session = session();
    session.evaluate("Setting.Clipping[0] = 0 / 0");
    assert_eq!(
        f64::from(0.1_f32),
        session.number("return Setting.Clipping[0]")
    );

    session.evaluate("Setting.Clipping[1] = 1 / 0");
    assert_eq!(
        f64::from(8.1_f32),
        session.number("return Setting.Clipping[1]")
    );

    session.evaluate("Setting.Clipping[2] = -1 / 0");
    assert_eq!(
        f64::from(8.1_f32),
        session.number("return Setting.Clipping[2]")
    );
}

/// REQ-PERS-087 read with SPEC-04 §4.2: the negative margins reach the conversion settings the
/// tile filter of REQ-E3D-002 and the visible rectangle of REQ-SCN-036 are built from.
#[test]
fn req_pers_087_a_negative_margin_reaches_the_conversion_settings() {
    let mut session = session();
    session.evaluate("Clipping.target = Color(-128, -80, -128, 0)");
    let clipping = session.engine.settings().clipping;
    assert_eq!(-128.0, clipping.x);
    assert_eq!(-80.0, clipping.y);
    assert_eq!(-128.0, clipping.z);
    assert_eq!(0.0, clipping.w);
}
