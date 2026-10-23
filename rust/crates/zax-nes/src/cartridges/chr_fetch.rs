/// Which pattern fetch a CHR access belongs to.
///
/// Only one required board cares: MMC5 holds two sets of CHR bank registers and, with 8x16
/// sprites selected, serves background fetches from one set and sprite fetches from the
/// other (nesdev, "MMC5"). Every other mapper ignores the distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ChrFetch {
    /// A background pattern fetch, a $2007 access or a host peek.
    #[default]
    Background = 0,
    /// A sprite pattern fetch.
    Sprite = 1,
}
