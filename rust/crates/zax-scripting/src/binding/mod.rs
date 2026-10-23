//! The binding layer: the host objects of SPEC-05 §4.2 and the value types of §4.3 as Lua
//! values, their member dispatch, argument checking and error raising.

pub(crate) mod access;
pub(crate) mod cx;
pub(crate) mod dispatch;
pub(crate) mod payload;
pub(crate) mod table;
