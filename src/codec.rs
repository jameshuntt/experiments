use crate::{ExperimentResult, ExperimentSpec, RESULT_SCHEMA, SCHEMA};
use core_1o::engine::{validate_ast, Lexer, Node, NodeValue, Parser};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::{fs, io, path::Path};

const EXPERIMENT_1O_SCHEMA: &str = include_str!("../schema/experiment/v1.schema.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    OneO,
    Json,
    Toml,
    Yaml,
}

impl Format {
    fn of(path: &Path) -> io::Result<Self> {
        if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
            if let Some(record) = core_1o::utils::parse_filename(name) {
                if record.modifier == "experiment" {
                    return Ok(Self::OneO);
                }
            }
        }
        match path.extension().and_then(|v| v.to_str()).map(str::to_ascii_lowercase).as_deref() {
            Some("1o") => Ok(Self::OneO),
            Some("json") => Ok(Self::Json),
            Some("toml") => Ok(Self::Toml),
            Some("yaml" | "yml") => Ok(Self::Yaml),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported experiment format {other:?}; use .1o.experiment, .1o, .json, .toml, .yaml or .yml"),
            )),
        }
    }
}

pub fn read_spec(path: impl AsRef<Path>) -> io::Result<ExperimentSpec> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)?;
    let spec: ExperimentSpec = match Format::of(path)? {
        Format::OneO => read_oneo_spec(&text)?,
        Format::Json => serde_json::from_str(&text).map_err(io::Error::other)?,
        Format::Toml => toml::from_str(&text).map_err(io::Error::other)?,
        Format::Yaml => serde_yaml::from_str(&text).map_err(io::Error::other)?,
    };
    if spec.schema != SCHEMA {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("experiment schema is `{}`, expected `{SCHEMA}`", spec.schema),
        ));
    }
    Ok(spec)
}

pub fn write_spec(path: impl AsRef<Path>, spec: &ExperimentSpec) -> io::Result<()> {
    let path = path.as_ref();
    let text = match Format::of(path)? {
        Format::OneO => write_oneo_spec(spec)?,
        Format::Json => serde_json::to_string_pretty(spec).map_err(io::Error::other)? + "\n",
        Format::Toml => toml::to_string_pretty(spec).map_err(io::Error::other)?,
        Format::Yaml => serde_yaml::to_string(spec).map_err(io::Error::other)?,
    };
    fs::write(path, text)
}

/// Results are deliberately stored as JSON by the runner. JSON/TOML/YAML are
/// supported for explicit result export/import; a result-specific 1o schema is
/// not faked by putting JSON behind a `.1o` suffix.
pub fn read_result(path: impl AsRef<Path>) -> io::Result<ExperimentResult> {
    let result: ExperimentResult = read_serde(path.as_ref())?;
    if result.schema != RESULT_SCHEMA {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "experiment result schema is `{}`, expected `{RESULT_SCHEMA}`",
                result.schema
            ),
        ));
    }
    Ok(result)
}

pub fn write_result(path: impl AsRef<Path>, result: &ExperimentResult) -> io::Result<()> {
    write_serde(path.as_ref(), result)
}

fn read_serde<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let text = fs::read_to_string(path)?;
    match Format::of(path)? {
        Format::Json => serde_json::from_str(&text).map_err(io::Error::other),
        Format::Toml => toml::from_str(&text).map_err(io::Error::other),
        Format::Yaml => serde_yaml::from_str(&text).map_err(io::Error::other),
        Format::OneO => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "1o result files require their own result schema; use JSON/TOML/YAML for results",
        )),
    }
}

fn write_serde<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let text = match Format::of(path)? {
        Format::Json => serde_json::to_string_pretty(value).map_err(io::Error::other)? + "\n",
        Format::Toml => toml::to_string_pretty(value).map_err(io::Error::other)?,
        Format::Yaml => serde_yaml::to_string(value).map_err(io::Error::other)?,
        Format::OneO => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "1o result files require their own result schema; use JSON/TOML/YAML for results",
            ))
        }
    };
    fs::write(path, text)
}

fn experiment_schema() -> io::Result<Value> {
    serde_json::from_str(EXPERIMENT_1O_SCHEMA).map_err(io::Error::other)
}

fn read_oneo_spec(text: &str) -> io::Result<ExperimentSpec> {
    let nodes = Parser::new(Lexer::new(text).tokenize())
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("1o parse: {error:?}")))?;
    let schema = experiment_schema()?;
    let mut diagnostics = Vec::new();
    validate_ast(&nodes, &schema, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("1o experiment schema: {diagnostics:?}"),
        ));
    }
    let root = entries_to_json(&nodes, &schema)?;
    let value = root
        .get("experiment")
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "1o experiment block is missing"))?;
    serde_json::from_value(value).map_err(io::Error::other)
}

fn write_oneo_spec(spec: &ExperimentSpec) -> io::Result<String> {
    let schema = experiment_schema()?;
    let mut root = Map::new();
    root.insert("experiment".into(), serde_json::to_value(spec).map_err(io::Error::other)?);
    let mut out = String::new();
    render_object(&Value::Object(root), &schema, 0, &mut out)?;
    Ok(out)
}

fn serde_key(oneo_key: &str, schema: &Value) -> String {
    schema
        .get("x-serde-key")
        .and_then(Value::as_str)
        .unwrap_or(oneo_key)
        .to_owned()
}

fn entries_to_json(nodes: &[Node], schema: &Value) -> io::Result<Value> {
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "1o schema object has no properties"))?;
    let mut object = Map::new();
    for (oneo_key, child_schema) in properties {
        let matches: Vec<&Node> = nodes.iter().filter(|node| node.key == *oneo_key).collect();
        let key = serde_key(oneo_key, child_schema);
        if matches.is_empty() {
            if child_schema.get("type").and_then(Value::as_str) == Some("array") {
                object.insert(key, Value::Array(Vec::new()));
            }
            continue;
        }
        if child_schema.get("type").and_then(Value::as_str) == Some("array") {
            let item_schema = child_schema.get("items").ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("array `{oneo_key}` has no item schema")))?;
            let mut values = Vec::with_capacity(matches.len());
            for node in matches {
                values.push(node_to_json(node, item_schema)?);
            }
            object.insert(key, Value::Array(values));
        } else {
            object.insert(key, node_to_json(matches[0], child_schema)?);
        }
    }
    Ok(Value::Object(object))
}

fn node_to_json(node: &Node, schema: &Value) -> io::Result<Value> {
    if schema.get("x-1o-kind").and_then(Value::as_str) == Some("criterion") {
        return criterion_to_json(node);
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => match &node.value {
            NodeValue::Block(children) => entries_to_json(children, schema),
            _ => Err(type_error(node, "block")),
        },
        Some("string") => match &node.value {
            NodeValue::String(value) => Ok(Value::String(value.clone())),
            _ => Err(type_error(node, "string")),
        },
        Some("integer") => match &node.value {
            NodeValue::Number(value) => Ok(Value::from(*value)),
            _ => Err(type_error(node, "integer")),
        },
        Some("number") => match &node.value {
            NodeValue::Number(value) => Ok(Value::from(*value)),
            NodeValue::Float(value) => Ok(Value::from(*value)),
            _ => Err(type_error(node, "number")),
        },
        Some("boolean") => match &node.value {
            NodeValue::Bool(value) => Ok(Value::Bool(*value)),
            _ => Err(type_error(node, "boolean")),
        },
        Some("array") => Err(io::Error::new(io::ErrorKind::InvalidData, "array schema used for a single 1o node")),
        _ if schema.get("x-1o-any-scalar").and_then(Value::as_bool) == Some(true) => scalar_node(node),
        _ => infer_node(node),
    }
}

fn type_error(node: &Node, expected: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, format!("1o key `{}` expected {expected}", node.key))
}

fn scalar_node(node: &Node) -> io::Result<Value> {
    match &node.value {
        NodeValue::String(value) => Ok(Value::String(value.clone())),
        NodeValue::Number(value) => Ok(Value::from(*value)),
        NodeValue::Float(value) => Ok(Value::from(*value)),
        NodeValue::Bool(value) => Ok(Value::Bool(*value)),
        NodeValue::Block(_) => Err(type_error(node, "scalar")),
    }
}

fn infer_node(node: &Node) -> io::Result<Value> {
    match &node.value {
        NodeValue::Block(children) => {
            let mut grouped: Map<String, Value> = Map::new();
            for child in children {
                let value = infer_node(child)?;
                match grouped.get_mut(&child.key) {
                    None => {
                        grouped.insert(child.key.clone(), value);
                    }
                    Some(existing) => match existing {
                        Value::Array(values) => values.push(value),
                        other => {
                            let first = std::mem::replace(other, Value::Null);
                            *other = Value::Array(vec![first, value]);
                        }
                    },
                }
            }
            Ok(Value::Object(grouped))
        }
        _ => scalar_node(node),
    }
}

fn criterion_to_json(node: &Node) -> io::Result<Value> {
    let NodeValue::Block(children) = &node.value else {
        return Err(type_error(node, "criterion block"));
    };
    let mut object = Map::new();
    let mut nested = Vec::new();
    for child in children {
        match child.key.as_str() {
            "criterion" => nested.push(criterion_to_json(child)?),
            "negated" => {
                object.insert("criterion".into(), criterion_to_json(child)?);
            }
            "baseline" | "candidate" | "reference" => {
                object.insert(child.key.clone(), infer_node(child)?);
            }
            "left" | "right" => {
                object.insert(child.key.clone(), infer_node(child)?);
            }
            _ => {
                object.insert(child.key.clone(), infer_node(child)?);
            }
        }
    }
    if !nested.is_empty() {
        object.insert("criteria".into(), Value::Array(nested));
    }
    Ok(Value::Object(object))
}

fn render_object(value: &Value, schema: &Value, indent: usize, out: &mut String) -> io::Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "1o writer expected object"))?;
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "1o schema object has no properties"))?;
    for (oneo_key, child_schema) in properties {
        let key = serde_key(oneo_key, child_schema);
        let Some(child) = object.get(&key) else { continue };
        if child.is_null() {
            continue;
        }
        if child_schema.get("type").and_then(Value::as_str) == Some("array") {
            let Some(items) = child.as_array() else {
                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("{key} is not an array")));
            };
            let item_schema = child_schema.get("items").ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("array `{oneo_key}` has no item schema")))?;
            for item in items {
                render_entry(oneo_key, item, item_schema, indent, out)?;
            }
        } else {
            render_entry(oneo_key, child, child_schema, indent, out)?;
        }
    }
    Ok(())
}

fn render_entry(key: &str, value: &Value, schema: &Value, indent: usize, out: &mut String) -> io::Result<()> {
    if schema.get("x-1o-kind").and_then(Value::as_str) == Some("criterion") {
        return render_criterion(key, value, indent, out);
    }
    let pad = " ".repeat(indent);
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            out.push_str(&format!("{pad}{key} {{\n"));
            render_object(value, schema, indent + 2, out)?;
            out.push_str(&format!("{pad}}}\n"));
        }
        Some("string") => out.push_str(&format!("{pad}{key} {}\n", quote_1o(value.as_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{key} is not a string")))?))),
        Some("integer") => out.push_str(&format!("{pad}{key} {}\n", value.as_i64().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{key} is not an integer")))?)),
        Some("number") => out.push_str(&format!("{pad}{key} {}\n", number_text(value)?)),
        Some("boolean") => out.push_str(&format!("{pad}{key} {}\n", value.as_bool().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{key} is not a boolean")))?)),
        _ if schema.get("x-1o-any-scalar").and_then(Value::as_bool) == Some(true) => out.push_str(&format!("{pad}{key} {}\n", scalar_text(value)?)),
        _ => render_inferred(key, value, indent, out)?,
    }
    Ok(())
}

fn render_criterion(key: &str, value: &Value, indent: usize, out: &mut String) -> io::Result<()> {
    let object = value.as_object().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "criterion is not an object"))?;
    let pad = " ".repeat(indent);
    out.push_str(&format!("{pad}{key} {{\n"));
    let inner = indent + 2;
    let inner_pad = " ".repeat(inner);
    for (name, child) in object {
        if child.is_null() { continue; }
        match name.as_str() {
            "criteria" => {
                for criterion in child.as_array().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "criteria is not an array"))? {
                    render_criterion("criterion", criterion, inner, out)?;
                }
            }
            "criterion" => render_criterion("negated", child, inner, out)?,
            _ => match child {
                Value::Object(_) => render_inferred(name, child, inner, out)?,
                _ => out.push_str(&format!("{inner_pad}{name} {}\n", scalar_text(child)?)),
            },
        }
    }
    out.push_str(&format!("{pad}}}\n"));
    Ok(())
}

fn render_inferred(key: &str, value: &Value, indent: usize, out: &mut String) -> io::Result<()> {
    let pad = " ".repeat(indent);
    match value {
        Value::Null => {}
        Value::Array(items) => {
            for item in items {
                render_inferred(key, item, indent, out)?;
            }
        }
        Value::Object(object) => {
            out.push_str(&format!("{pad}{key} {{\n"));
            for (child_key, child) in object {
                render_inferred(child_key, child, indent + 2, out)?;
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        _ => out.push_str(&format!("{pad}{key} {}\n", scalar_text(value)?)),
    }
    Ok(())
}

fn scalar_text(value: &Value) -> io::Result<String> {
    match value {
        Value::String(value) => Ok(quote_1o(value)),
        Value::Number(_) => number_text(value),
        Value::Bool(value) => Ok(value.to_string()),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "1o scalar expected")),
    }
}

fn number_text(value: &Value) -> io::Result<String> {
    value
        .as_number()
        .map(ToString::to_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "number expected"))
}

fn quote_1o(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ExperimentTarget, MeasurementPlan, MutationPlan, PromotionPolicy, Protocol, Verification,
    };
    use scientific_method::{Hypothesis, ScientificMethod};

    fn minimal() -> ExperimentSpec {
        ExperimentSpec {
            schema: SCHEMA.to_owned(),
            id: "codec".into(),
            description: Some("same semantic experiment in every format".into()),
            scientific_method: ScientificMethod::new(Hypothesis::new("candidate changes")),
            target: ExperimentTarget { root: ".".into(), label: None },
            protocol: Protocol::default(),
            mutation: MutationPlan::default(),
            verification: Verification::default(),
            measurements: MeasurementPlan::default(),
            differences: Vec::new(),
            promotion: PromotionPolicy::default(),
        }
    }

    #[test]
    fn oneo_is_real_oneo_and_round_trips() {
        let spec = minimal();
        let text = write_oneo_spec(&spec).expect("write 1o");
        assert!(text.starts_with("experiment {\n"));
        assert!(!text.trim_start().starts_with('{'));
        let back = read_oneo_spec(&text).expect("read 1o");
        assert_eq!(back, spec);
    }
}
