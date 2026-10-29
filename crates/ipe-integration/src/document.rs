//! Bounded YAML decoding shared by native integration readers.
use serde_json::{json, Value};
type Result<T> = std::result::Result<T, &'static str>;
const YAML_LIMIT: usize = 48 * 1024;

// Use parser events rather than an expanding YAML loader. Bound depth and nodes
// before materialization; reject aliases, tags, merge keys, and duplicate keys.
pub(crate) fn parse_yaml_located(source: &str) -> Result<(Value, Value)> {
    use yaml_rust2::{
        parser::{Event, Parser},
        scanner::TScalarStyle,
        Yaml,
    };
    if source.len() > YAML_LIMIT {
        return Err("yaml_byte_limit");
    }
    fn node(
        parser: &mut Parser<std::str::Chars<'_>>,
        depth: usize,
        nodes: &mut usize,
        pointer: &str,
        locations: &mut serde_json::Map<String, Value>,
    ) -> Result<Value> {
        *nodes += 1;
        if depth > 24 || *nodes > 8192 {
            return Err("yaml_structure_limit");
        }
        let (event, mark) = parser.next_token().map_err(|_| "invalid_yaml")?;
        locations
            .entry(pointer.to_owned())
            .or_insert_with(|| json!({"line":mark.line(),"column":mark.col()+1}));
        match event {
            Event::Scalar(text, style, 0, None) => {
                if style != TScalarStyle::Plain {
                    return Ok(json!(text));
                }
                Ok(match Yaml::from_str(&text) {
                    Yaml::String(v) => json!(v),
                    Yaml::Integer(v) => json!(v),
                    Yaml::Boolean(v) => json!(v),
                    Yaml::Null => Value::Null,
                    Yaml::Real(v) => {
                        let n = v.parse::<f64>().map_err(|_| "invalid_yaml_number")?;
                        if !n.is_finite() {
                            return Err("invalid_yaml_number");
                        }
                        json!(n)
                    },
                    _ => return Err("unsupported_yaml_scalar"),
                })
            },
            Event::MappingStart(0, None) => {
                let mut object = serde_json::Map::new();
                loop {
                    if matches!(parser.peek().map_err(|_| "invalid_yaml")?.0, Event::MappingEnd) {
                        parser.next_token().map_err(|_| "invalid_yaml")?;
                        break;
                    }
                    let key = node(parser, depth + 1, nodes, pointer, locations)?;
                    let key = key.as_str().ok_or("yaml_non_string_key")?.to_owned();
                    if key == "<<" {
                        return Err("yaml_merge_unsupported");
                    }
                    if object.contains_key(&key) {
                        return Err("yaml_duplicate_key");
                    }
                    let child = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                    object.insert(key, node(parser, depth + 1, nodes, &child, locations)?);
                }
                Ok(Value::Object(object))
            },
            Event::SequenceStart(0, None) => {
                let mut values = Vec::new();
                loop {
                    if matches!(parser.peek().map_err(|_| "invalid_yaml")?.0, Event::SequenceEnd) {
                        parser.next_token().map_err(|_| "invalid_yaml")?;
                        break;
                    }
                    values.push(node(
                        parser,
                        depth + 1,
                        nodes,
                        &format!("{pointer}/{}", values.len()),
                        locations,
                    )?);
                }
                Ok(Value::Array(values))
            },
            Event::Alias(_) => Err("yaml_alias_unsupported"),
            Event::Scalar(_, _, _, _) | Event::MappingStart(_, _) | Event::SequenceStart(_, _) => {
                Err("yaml_anchor_or_tag_unsupported")
            },
            _ => Err("invalid_yaml"),
        }
    }
    let mut parser = Parser::new_from_str(source);
    for expected in [Event::StreamStart, Event::DocumentStart] {
        if parser.next_token().map_err(|_| "invalid_yaml")?.0 != expected {
            return Err("yaml_document_required");
        }
    }
    let mut locations = serde_json::Map::new();
    let result = node(&mut parser, 0, &mut 0, "", &mut locations)?;
    for expected in [Event::DocumentEnd, Event::StreamEnd] {
        if parser.next_token().map_err(|_| "invalid_yaml")?.0 != expected {
            return Err("single_yaml_document_required");
        }
    }
    if !result.is_object() {
        return Err("yaml_object_required");
    }
    if serde_json::to_vec(&result).map_err(|_| "invalid_yaml")?.len() > 60 * 1024 {
        return Err("yaml_projection_limit");
    }
    Ok((result, Value::Object(locations)))
}
