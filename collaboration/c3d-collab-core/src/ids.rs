use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Connected collaboration client identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ClientId(Ulid);

/// Anchored scene comment identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommentId(Ulid);

/// Branch/proposal identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProposalId(Ulid);

macro_rules! impl_id {
    ($name:ident) => {
        impl $name {
            /// Create a new random identifier.
            pub fn new() -> Self {
                Self(Ulid::new())
            }

            /// Parse a canonical string representation.
            pub fn parse(value: &str) -> Result<Self, String> {
                Ulid::from_string(value)
                    .map(Self)
                    .map_err(|err| err.to_string())
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
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
    };
}

impl_id!(ClientId);
impl_id!(CommentId);
impl_id!(ProposalId);
