//! Insertion-ordered JSON for signed bytes.
//!
//! A JS object lists its keys in ECMAScript `[[OwnPropertyKeys]]` order: the canonical array
//! indices (`"0"` … `"4294967294"`) ascending, then every other key in insertion order. JS signs
//! token-unit metas and policies as `JSON.stringify` of such objects, so a Rust port must keep
//! that order. `serde_json::Value` cannot: without its `preserve_order` feature (which would
//! change the map type for every crate that links `serde_json`) its objects are key-sorted.
//! [`OrderedValue`] keeps document order when it is parsed and writes every object in JS order.

use indexmap::IndexMap;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;

/// An insertion-ordered map with string keys.
pub type OrderedMap<V = OrderedValue> = IndexMap<String, V>;

/// A policy: action (`read` / `write`) → meta key → permissions, in JS key order.
pub type PolicyMap = OrderedMap<OrderedMap<Vec<String>>>;

/// A JSON value whose objects keep their key order (see the module docs).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OrderedValue {
    #[default]
    Null,
    Bool(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<OrderedValue>),
    Object(OrderedMap<OrderedValue>),
}

impl OrderedValue {
    /// The string, if this is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            OrderedValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// The number as `f64`, if this is a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            OrderedValue::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    pub fn as_array(&self) -> Option<&Vec<OrderedValue>> {
        match self {
            OrderedValue::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The entries, if this is an object.
    pub fn as_object(&self) -> Option<&OrderedMap<OrderedValue>> {
        match self {
            OrderedValue::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Whether this is JSON null.
    pub fn is_null(&self) -> bool {
        matches!(self, OrderedValue::Null)
    }

    /// The compact JSON text, objects in JS key order (JS `JSON.stringify`).
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// The canonical array index a JS object key names: decimal digits without leading zeros
/// (`"0"` itself excepted), `0..=4294967294`. Such keys come first in a JS object.
pub(crate) fn js_array_index(key: &str) -> Option<u32> {
    if key.is_empty()
        || !key.bytes().all(|b| b.is_ascii_digit())
        || (key.len() > 1 && key.starts_with('0'))
    {
        return None;
    }
    key.parse::<u32>().ok().filter(|&index| index < u32::MAX)
}

/// JS object key order: array indices ascending before every other key; other keys compare
/// equal, so a stable sort keeps their insertion order.
fn js_key_cmp(a: &str, b: &str) -> Ordering {
    match (js_array_index(a), js_array_index(b)) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Reorders `map` as a JS object lists it: array-index keys ascending, then the rest in
/// insertion order (a stable sort).
pub(crate) fn sort_js_keys<V>(map: &mut OrderedMap<V>) {
    map.sort_by(|a, _, b, _| js_key_cmp(a, b));
}

/// Serializes `map` as JS `JSON.stringify` writes an object: array-index keys ascending, then
/// the rest in insertion order. Usable as `#[serde(serialize_with = …)]`.
pub(crate) fn serialize_js_object<V: Serialize, S: Serializer>(
    map: &OrderedMap<V>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut entries: Vec<(&String, &V)> = map.iter().collect();
    entries.sort_by(|a, b| js_key_cmp(a.0, b.0));
    let mut out = serializer.serialize_map(Some(entries.len()))?;
    for (key, value) in entries {
        out.serialize_entry(key, value)?;
    }
    out.end()
}

impl Serialize for OrderedValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            OrderedValue::Null => serializer.serialize_unit(),
            OrderedValue::Bool(b) => serializer.serialize_bool(*b),
            OrderedValue::Number(n) => n.serialize(serializer),
            OrderedValue::String(s) => serializer.serialize_str(s),
            OrderedValue::Array(items) => items.serialize(serializer),
            OrderedValue::Object(map) => serialize_js_object(map, serializer),
        }
    }
}

impl<'de> Deserialize<'de> for OrderedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedValueVisitor;

        impl<'de> Visitor<'de> for OrderedValueVisitor {
            type Value = OrderedValue;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any JSON value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<OrderedValue, E> {
                Ok(OrderedValue::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<OrderedValue, E> {
                Ok(OrderedValue::Number(v.into()))
            }

            fn visit_u64<E>(self, v: u64) -> Result<OrderedValue, E> {
                Ok(OrderedValue::Number(v.into()))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<OrderedValue, E> {
                serde_json::Number::from_f64(v)
                    .map(OrderedValue::Number)
                    .ok_or_else(|| E::custom("a JSON number must be finite"))
            }

            fn visit_str<E>(self, v: &str) -> Result<OrderedValue, E> {
                Ok(OrderedValue::String(v.to_string()))
            }

            fn visit_string<E>(self, v: String) -> Result<OrderedValue, E> {
                Ok(OrderedValue::String(v))
            }

            fn visit_unit<E>(self) -> Result<OrderedValue, E> {
                Ok(OrderedValue::Null)
            }

            fn visit_none<E>(self) -> Result<OrderedValue, E> {
                Ok(OrderedValue::Null)
            }

            fn visit_some<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<OrderedValue, D::Error> {
                OrderedValue::deserialize(deserializer)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<OrderedValue, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(OrderedValue::Array(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<OrderedValue, A::Error> {
                // Document order; a repeated key keeps its first position and its last value,
                // as JSON.parse does
                let mut map = OrderedMap::new();
                while let Some((key, value)) = access.next_entry::<String, OrderedValue>()? {
                    map.insert(key, value);
                }
                Ok(OrderedValue::Object(map))
            }
        }

        deserializer.deserialize_any(OrderedValueVisitor)
    }
}

impl From<serde_json::Value> for OrderedValue {
    /// A `serde_json::Value` object is already key-sorted, so its order is the sorted one.
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => OrderedValue::Null,
            serde_json::Value::Bool(b) => OrderedValue::Bool(b),
            serde_json::Value::Number(n) => OrderedValue::Number(n),
            serde_json::Value::String(s) => OrderedValue::String(s),
            serde_json::Value::Array(items) => {
                OrderedValue::Array(items.into_iter().map(Into::into).collect())
            }
            serde_json::Value::Object(map) => {
                OrderedValue::Object(map.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl From<OrderedValue> for serde_json::Value {
    fn from(value: OrderedValue) -> Self {
        match value {
            OrderedValue::Null => serde_json::Value::Null,
            OrderedValue::Bool(b) => serde_json::Value::Bool(b),
            OrderedValue::Number(n) => serde_json::Value::Number(n),
            OrderedValue::String(s) => serde_json::Value::String(s),
            OrderedValue::Array(items) => {
                serde_json::Value::Array(items.into_iter().map(Into::into).collect())
            }
            OrderedValue::Object(map) => {
                serde_json::Value::Object(map.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
        }
    }
}

impl From<&str> for OrderedValue {
    fn from(value: &str) -> Self {
        OrderedValue::String(value.to_string())
    }
}

impl From<String> for OrderedValue {
    fn from(value: String) -> Self {
        OrderedValue::String(value)
    }
}

impl From<Vec<OrderedValue>> for OrderedValue {
    fn from(items: Vec<OrderedValue>) -> Self {
        OrderedValue::Array(items)
    }
}

impl From<OrderedMap<OrderedValue>> for OrderedValue {
    fn from(map: OrderedMap<OrderedValue>) -> Self {
        OrderedValue::Object(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_indices_are_canonical_u32_below_the_maximum() {
        assert_eq!(js_array_index("0"), Some(0));
        assert_eq!(js_array_index("10"), Some(10));
        assert_eq!(js_array_index("4294967294"), Some(4_294_967_294));
        for key in [
            "",
            "01",
            "-1",
            "1.0",
            "+1",
            " 1",
            "4294967295",
            "99999999999",
            "a1",
        ] {
            assert_eq!(js_array_index(key), None, "{key:?}");
        }
    }

    #[test]
    fn objects_serialize_indices_first_then_insertion_order() -> serde_json::Result<()> {
        let value: OrderedValue = serde_json::from_str(
            r#"{"z":1,"10":2,"2":3,"a":{"b":1,"01":2,"0":3},"4294967295":4}"#,
        )?;
        assert_eq!(
            serde_json::to_string(&value)?,
            r#"{"2":3,"10":2,"z":1,"a":{"0":3,"b":1,"01":2},"4294967295":4}"#
        );
        Ok(())
    }

    #[test]
    fn parsing_keeps_document_order_and_sort_js_keys_is_stable() -> serde_json::Result<()> {
        let value: OrderedValue = serde_json::from_str(r#"{"b":1,"3":2,"a":3,"1":4}"#)?;
        let mut map = value.as_object().cloned().unwrap_or_default();
        assert_eq!(
            map.keys().map(String::as_str).collect::<Vec<_>>(),
            ["b", "3", "a", "1"]
        );
        sort_js_keys(&mut map);
        assert_eq!(
            map.keys().map(String::as_str).collect::<Vec<_>>(),
            ["1", "3", "b", "a"]
        );
        Ok(())
    }

    #[test]
    fn converts_to_and_from_serde_json_values() {
        let json = serde_json::json!({ "a": [1, 2.5, "x", null, true], "b": { "c": false } });
        let ordered = OrderedValue::from(json.clone());
        assert_eq!(serde_json::Value::from(ordered), json);
    }
}
