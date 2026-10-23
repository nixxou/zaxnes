/// Equality to `decimals` decimal places, as the precision overload of xUnit's `Assert.Equal`.
pub fn assert_close(actual: f32, expected: f32, decimals: i32) {
    let tolerance = 0.5 * 10f32.powi(-decimals);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} is not {expected} to {decimals} decimal places"
    );
}
