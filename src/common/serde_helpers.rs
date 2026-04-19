use serde::{Deserialize, Deserializer};

/// Deserialises a JSON field into `Option<Option<T>>`:
/// - field absent  → `None`          (caller: do not touch this field)
/// - field `null`  → `Some(None)`    (caller: clear this field)
/// - field value   → `Some(Some(v))` (caller: set this field)
///
/// Use with `#[serde(default, deserialize_with = "crate::common::serde_helpers::nullable")]`.
pub fn nullable<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::<T>::deserialize(d)?))
}
