//! Proves that the interpreter loads and that the language level the embedded profiles need is
//! actually available.
//!
//! PLAN.md section 4, decision of 2026-10-20: Lua 5.4 was chosen over `MoonSharp` because the
//! inventory of the 579 scripts found `goto` and labels in 2 profiles, which requires Lua 5.2
//! or later. These tests are the executable form of that decision.
//!
//! The C# test `Disposed_runtime_rejects_further_use` has no counterpart: a runtime that has
//! been dropped cannot be named, so the state it checks cannot arise.

use zax_scripting::LuaRuntime;

#[test]
fn native_library_loads_and_reports_lua_5_4() {
    let runtime = LuaRuntime::new();

    assert_eq!(runtime.language_version().unwrap(), "Lua 5.4");
}

/// The feature the runtime choice hinges on: a forward `goto` to a `::continue::` label at the
/// end of a loop body. Rejected outright by Lua 5.1 and by `MoonSharp` 5.2 era grammars that omit
/// labels.
#[test]
fn goto_and_labels_are_supported() {
    let runtime = LuaRuntime::new();

    let chunk = r"
        local sum = 0
        for i = 1, 10 do
          if i % 2 == 0 then goto continue end
          sum = sum + i
          ::continue::
        end
        return sum
        ";

    // 1 + 3 + 5 + 7 + 9
    assert_eq!(runtime.evaluate_integer(chunk).unwrap(), 25);
}

#[test]
fn goto_can_jump_out_of_nested_loops() {
    let runtime = LuaRuntime::new();

    let chunk = r"
        local visited = 0
        for i = 1, 4 do
          for j = 1, 4 do
            if i * j > 6 then goto done end
            visited = visited + 1
          end
        end
        ::done::
        return visited
        ";

    // (1,1..4) and (2,1..3) are visited, then i*j = 8 jumps out: 4 + 3 = 7.
    assert_eq!(runtime.evaluate_integer(chunk).unwrap(), 7);
}

#[test]
fn each_runtime_owns_an_independent_state() {
    let first = LuaRuntime::new();
    let second = LuaRuntime::new();

    first.execute("shared = 'first'").unwrap();

    assert_eq!(
        second.evaluate_string("return type(shared)").unwrap(),
        "nil"
    );
}
