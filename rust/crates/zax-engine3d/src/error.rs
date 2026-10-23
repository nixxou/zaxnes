//! Errors of the conversion engine.

/// Why the engine refused an operation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    /// A profile pattern declares a grid size the engine cannot hold.
    #[error("a pattern grid of {width} x {height} cells is not valid")]
    PatternSize {
        /// Declared width.
        width: i32,
        /// Declared height.
        height: i32,
    },
    /// The instance given to a script call belongs to no registry pattern.
    #[error("the instance does not belong to a registry pattern")]
    NotARegistryInstance,
    /// A learned-state block is not valid (DEC-R2-T5).
    #[error("{0}")]
    InvalidState(String),
}
