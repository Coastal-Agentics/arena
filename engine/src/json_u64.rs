//! JS-safe JSON for `u64` values (seeds): written as decimal strings, because JavaScript
//! numbers lose precision above 2^53. Reading accepts a string or a plain number, so
//! older JSON (numeric seeds) still loads. Use with `#[serde(with = "engine::json_u64")]`.

use serde::de::{self, Visitor};
use serde::{Deserializer, Serializer};
use std::fmt;

pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
    s.collect_str(v)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    struct U64Visitor;
    impl Visitor<'_> for U64Visitor {
        type Value = u64;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a u64 as a decimal string or a non-negative integer")
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u64, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u64, E> {
            u64::try_from(v).map_err(|_| E::custom(format!("negative seed {v}")))
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<u64, E> {
            v.parse()
                .map_err(|_| E::custom(format!("invalid u64 string {v:?}")))
        }
    }
    d.deserialize_any(U64Visitor)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct S {
        #[serde(with = "super")]
        seed: u64,
    }

    #[test]
    fn writes_string_reads_both() {
        let max = S { seed: u64::MAX };
        let json = serde_json::to_string(&max).unwrap();
        assert_eq!(json, r#"{"seed":"18446744073709551615"}"#);
        assert_eq!(serde_json::from_str::<S>(&json).unwrap(), max);
        // Backward compat: plain numbers still load, even above 2^53.
        assert_eq!(
            serde_json::from_str::<S>(r#"{"seed":18446744073709551615}"#).unwrap(),
            max
        );
        assert_eq!(
            serde_json::from_str::<S>(r#"{"seed":42}"#).unwrap(),
            S { seed: 42 }
        );
        assert!(serde_json::from_str::<S>(r#"{"seed":-1}"#).is_err());
        assert!(serde_json::from_str::<S>(r#"{"seed":"x"}"#).is_err());
        assert!(serde_json::from_str::<S>(r#"{"seed":1.5}"#).is_err());
    }
}
