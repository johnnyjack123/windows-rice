//! Settings patch semantics, identical to `applyPatch()` in
//! `web/src/backend/settings.ts`.

use serde_json::{Map, Value};

/// Applies `patch` to `base`: objects merge key by key, arrays and primitives
/// replace, `null` deletes the key. A non-object patch leaves `base` as it is.
pub fn apply_patch(base: &Value, patch: &Value) -> Value {
    let Value::Object(patch) = patch else {
        return base.clone();
    };
    let mut out = match base {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    for (key, value) in patch {
        match value {
            Value::Null => {
                out.remove(key);
            }
            Value::Object(_) => {
                let merged = match out.get(key) {
                    Some(existing @ Value::Object(_)) => apply_patch(existing, value),
                    _ => apply_patch(&Value::Object(Map::new()), value),
                };
                out.insert(key.clone(), merged);
            }
            _ => {
                out.insert(key.clone(), value.clone());
            }
        }
    }
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merges_objects_replaces_arrays_and_deletes_on_null() {
        let base = json!({ "panel": { "sections": ["a", "b"], "lastSection": "a" }, "theme": { "overrides": { "x": { "speed": 2 } } } });
        let out = apply_patch(&base, &json!({ "panel": { "sections": ["c"] }, "theme": { "overrides": { "x": null } } }));
        assert_eq!(out, json!({ "panel": { "sections": ["c"], "lastSection": "a" }, "theme": { "overrides": {} } }));
        assert_eq!(base["panel"]["sections"], json!(["a", "b"]));
    }

    #[test]
    fn drops_nulls_inside_new_objects() {
        let out = apply_patch(&json!({ "island": 3 }), &json!({ "island": { "peek": false, "hotkey": null } }));
        assert_eq!(out, json!({ "island": { "peek": false } }));
    }

    #[test]
    fn ignores_non_object_patches_and_bases() {
        let base = json!({ "locale": "de" });
        assert_eq!(apply_patch(&base, &json!(null)), base);
        assert_eq!(apply_patch(&base, &json!([1, 2])), base);
        assert_eq!(apply_patch(&json!("broken"), &json!({ "locale": "en" })), json!({ "locale": "en" }));
    }

    #[test]
    fn applying_in_order_is_associative_for_nested_objects() {
        let a = json!({ "theme": { "overrides": { "x": { "speed": 1.5 } } } });
        let b = json!({ "theme": { "overrides": { "x": { "bounce": 0.1 } } }, "locale": "en" });
        let stepwise = apply_patch(&apply_patch(&json!({}), &a), &b);
        assert_eq!(stepwise, json!({ "theme": { "overrides": { "x": { "speed": 1.5, "bounce": 0.1 } } }, "locale": "en" }));
    }
}
