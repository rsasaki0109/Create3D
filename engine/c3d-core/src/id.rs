//! Stable identifier types for scene and asset objects.

use std::fmt;
use std::str::FromStr;

use ulid::Ulid;

use crate::error::{C3dError, C3dResult};

macro_rules! define_id {
    ($name:ident) => {
        /// Stable sortable identifier.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(Ulid);

        impl $name {
            /// Create a new random identifier.
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            /// Parse a canonical string representation.
            pub fn parse(value: &str) -> C3dResult<Self> {
                Ulid::from_string(value)
                    .map(Self)
                    .map_err(|err| C3dError::InvalidId(err.to_string()))
            }

            /// Returns the canonical string representation.
            pub fn as_str(&self) -> String {
                self.0.to_string()
            }

            /// Returns the underlying bytes.
            pub fn to_bytes(self) -> [u8; 16] {
                self.0.to_bytes()
            }

            /// Reconstruct from raw bytes.
            pub fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(Ulid::from_bytes(bytes))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = C3dError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

define_id!(EntityId);
define_id!(AssetId);
define_id!(OperationId);
define_id!(TransactionId);

/// Generates monotonic ULIDs suitable for operation logs.
#[derive(Default)]
pub struct UlidGenerator {
    generator: ulid::Generator,
}

impl fmt::Debug for UlidGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UlidGenerator").finish_non_exhaustive()
    }
}

impl UlidGenerator {
    /// Create a new generator.
    pub fn new() -> Self {
        Self {
            generator: ulid::Generator::new(),
        }
    }

    /// Generate the next identifier.
    pub fn next_entity_id(&mut self) -> EntityId {
        EntityId(self.generator.generate().expect("ulid generation"))
    }

    /// Generate the next asset identifier.
    pub fn next_asset_id(&mut self) -> AssetId {
        AssetId(self.generator.generate().expect("ulid generation"))
    }

    /// Generate the next operation identifier.
    pub fn next_operation_id(&mut self) -> OperationId {
        OperationId(self.generator.generate().expect("ulid generation"))
    }

    /// Generate the next transaction identifier.
    pub fn next_transaction_id(&mut self) -> TransactionId {
        TransactionId(self.generator.generate().expect("ulid generation"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_round_trip() {
        let id = EntityId::new();
        let parsed = EntityId::parse(&id.to_string()).expect("parse id");
        assert_eq!(id, parsed);
    }

    #[test]
    fn generator_is_monotonic() {
        let mut generator = UlidGenerator::new();
        let first = generator.next_entity_id();
        let second = generator.next_entity_id();
        assert!(first < second);
    }
}
