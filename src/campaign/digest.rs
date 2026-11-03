//! Every digest a campaign takes (an experiment spec's, a plan's, a result file's): stable_digest's SHA-256 over one
//! canonical byte form, declared here once:
//!
//! ```text
//! experiments.canonical.v1\n<what>\n<the value as JSON, object keys sorted, no whitespace>
//! ```
//!
//! `what` names the thing digested, so two different things with the same JSON never share a digest. A digest is
//! written `sha256:<64 hex>`.

use serde::Serialize;
use serde_json::Value;
use stable_digest::StableDigest;

/// The canonical form's first line.
pub const CANONICAL: &str = "experiments.canonical.v1";

/// The canonical bytes of `value`, as `what`.
pub fn canonical<T: Serialize + ?Sized>(what: &str, value: &T) -> Vec<u8> {
    let v = serde_json::to_value(value).unwrap_or(Value::Null);
    let mut out = format!("{CANONICAL}\n{what}\n").into_bytes();
    write_sorted(&v, &mut out);
    out
}

fn write_sorted(v: &Value, out: &mut Vec<u8>) {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push(b'{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                out.extend(serde_json::to_vec(k).unwrap_or_default());
                out.push(b':');
                write_sorted(&map[k.as_str()], out);
            }
            out.push(b'}');
        }
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_sorted(item, out);
            }
            out.push(b']');
        }
        other => out.extend(serde_json::to_vec(other).unwrap_or_default()),
    }
}

/// The digest of `value` as `what`.
pub fn of<T: Serialize + ?Sized>(what: &str, value: &T) -> String {
    tagged(&canonical(what, value))
}

/// The digest of bytes as they are (a file's).
pub fn bytes(material: &[u8]) -> String {
    tagged(material)
}

fn tagged(material: &[u8]) -> String {
    format!("sha256:{}", StableDigest::sha256(material).as_str())
}
