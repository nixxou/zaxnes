//! P0 gate checks for `zax-scripting`. No `REQ-` identifier is referenced because no SPEC-05
//! requirement existed yet.

use zax_scripting::{LuaRuntime, MODULE_NAME, MODULE_ROLE};

#[test]
fn module_is_identified() {
    assert_eq!(MODULE_NAME, "Zax.Scripting");
    assert!(!MODULE_ROLE.trim().is_empty());
}

#[test]
fn runtime_reports_an_error_for_a_malformed_chunk() {
    let runtime = LuaRuntime::new();

    let error = runtime.execute("this is not lua").unwrap_err();
    assert!(!error.to_string().trim().is_empty());
}
