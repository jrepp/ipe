//! Strict JSON decoding for untrusted input. This wraps serde_json rather than
//! implementing JSON syntax: a recursive visitor rejects duplicate decoded keys
//! before typed deserialization. Errors never retain input, field names, or values.
//!
//! Callers must enforce byte limits before decoding. serde_json's default depth
//! limit remains enabled. A typed map-only duplicate-key helper cannot replace
//! this boundary: duplicates can occur in nested objects, arrays, and Value fields
//! before the final request schema is known. Typed `deny_unknown_fields` and domain
//! validation remain the caller's responsibility; this crate does not add them.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt;

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "JSON without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(v)))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(v.into())))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(v.into())))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| StrictValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v.to_owned())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v)))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(v)) = seq.next_element()? {
                    values.push(v);
                }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate object key"));
                    }
                    let StrictValue(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

/// An intentionally opaque error: exposing serde diagnostics can leak raw keys,
/// values or request details through logs and API error messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid JSON")
    }
}
impl std::error::Error for Error {}

/// Decode a complete JSON document, rejecting duplicate keys at every depth.
pub fn parse_value(bytes: &[u8]) -> Result<Value, Error> {
    serde_json::from_slice::<StrictValue>(bytes).map(|v| v.0).map_err(|_| Error)
}

/// Strictly decode, then deserialize into an owned request type. Both syntax and
/// schema failures produce the same sanitized error, without an underlying cause.
pub fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    serde_json::from_value(parse_value(bytes)?).map_err(|_| Error)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_keys_recursively_including_arrays_and_escapes() {
        for input in [
            r#"{"secret.internal":1,"secret.internal":2}"#,
            r#"{"nested":{"role":"user","role":"admin"}}"#,
            r#"[{"nested":[{"key":1,"key":2}]}]"#,
            r#"{"key":1,"\u006bey":2}"#,
        ] {
            assert_eq!(parse_value(input.as_bytes()), Err(Error));
        }
    }
    #[test]
    fn rejects_trailing_invalid_and_excessively_deep_input() {
        for input in
            [b"{} {}".as_slice(), b"[1,]", b"{\"value\":NaN}", b"\xff", b"", b"{\"x\":1e999}"]
        {
            assert_eq!(parse_value(input), Err(Error));
        }
        let deep = format!("{}0{}", "[".repeat(256), "]".repeat(256));
        assert_eq!(parse_value(deep.as_bytes()), Err(Error));
    }
    #[derive(Debug, serde::Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Request {
        name: String,
        metadata: Value,
    }
    #[test]
    fn typed_deserialization_and_value_api_preserve_valid_data() {
        let input = br#"{"name":"agent","metadata":[true,1,-2,1.5,"text",null,{"x":1}]}"#;
        let value = parse_value(input).unwrap();
        assert_eq!(
            parse::<Request>(input).unwrap(),
            Request {
                name: "agent".into(),
                metadata: value["metadata"].clone()
            }
        );
        assert_eq!(parse::<Value>(input).unwrap(), value);
        assert_eq!(parse_value(b"{} \n\t").unwrap(), serde_json::json!({}));
    }
    #[test]
    fn errors_never_disclose_keys_values_or_schema_details() {
        for input in [
            br#"{"secret.internal":1,"secret.internal":2}"#.as_slice(),
            br#"{"name":"ok","metadata":null,"secret.internal":"secret-value"}"#,
            br#"{"name":{"secret.internal":"secret-value"},"metadata":null}"#,
        ] {
            let error = parse::<Request>(input).unwrap_err();
            assert_eq!(error.to_string(), "invalid JSON");
            assert_eq!(format!("{error:?}"), "Error");
            assert!(std::error::Error::source(&error).is_none());
        }
    }
}
