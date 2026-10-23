/// Exact equality of two floats, as the C# `Assert.Equal(float, float)`: the values are the ones the
/// settings hold, not the results of arithmetic.
#[allow(clippy::float_cmp, reason = "the C# test asserts these values exactly")]
pub fn assert_exact(actual: f32, expected: f32) {
    assert_eq!(actual, expected);
}
