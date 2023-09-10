use std::borrow::Cow;
use std::fmt::Debug;

use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// The language server protocol always uses “2.0” as the jsonrpc version. [`abstractMessage`]
///
/// [`abstractMessage`]: https://microsoft.github.io/language-server-protocol/specification#abstractMessage
#[derive(PartialEq)]
pub struct Version;
const JSON_RPC_VERSION_STR: &str = "2.0";

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", JSON_RPC_VERSION_STR)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(JSON_RPC_VERSION_STR)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cow_str = Cow::<'de, str>::deserialize(deserializer)?;

        match cow_str.as_ref() {
            JSON_RPC_VERSION_STR => Ok(Version),
            version => Err(de::Error::unknown_variant(version, &[JSON_RPC_VERSION_STR])),
        }
    }
}
