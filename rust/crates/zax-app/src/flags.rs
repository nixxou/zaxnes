//! A small flag-set type generator: the C# `[Flags]` enums of the input layer.

/// Declares a newtype over an unsigned integer with named bits, the usual bit operators,
/// `contains` and `is_empty`.
macro_rules! flag_set {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident($ty:ty) {
            $( $(#[$flag_meta:meta])* const $flag:ident = $value:expr; )*
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        $vis struct $name($ty);

        impl $name {
            /// No bit set.
            pub const NONE: Self = Self(0);
            $( $(#[$flag_meta])* pub const $flag: Self = Self($value); )*

            /// The raw bits.
            #[must_use]
            pub const fn bits(self) -> $ty {
                self.0
            }

            /// The set made of the raw `bits`.
            #[must_use]
            pub const fn from_bits(bits: $ty) -> Self {
                Self(bits)
            }

            /// Whether every bit of `other` is set.
            #[must_use]
            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            /// Whether at least one bit of `other` is set.
            #[must_use]
            pub const fn intersects(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }

            /// Whether no bit is set.
            #[must_use]
            pub const fn is_empty(self) -> bool {
                self.0 == 0
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }

        impl std::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }
        }

        impl std::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, other: Self) {
                self.0 |= other.0;
            }
        }

        impl std::ops::BitAndAssign for $name {
            fn bitand_assign(&mut self, other: Self) {
                self.0 &= other.0;
            }
        }
    };
}

pub(crate) use flag_set;
