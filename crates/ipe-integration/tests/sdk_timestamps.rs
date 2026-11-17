//! Wire compatibility checks for the temporary MCP timestamp dependency patch.
use jiff::Timestamp;
use rmcp::model::Annotations;
use serde_json::json;

#[test]
fn typed_annotations_keep_rfc3339_instants_and_json_round_trips() {
    for text in ["2025-01-12T15:00:58Z", "2025-01-12T16:00:58+01:00", "1969-12-31T23:59:59.5Z"] {
        let timestamp: Timestamp = text.parse().unwrap();
        let annotations = Annotations::for_resource(0.5, timestamp);
        let value = serde_json::to_value(&annotations).unwrap();
        let wire_timestamp: Timestamp = value["lastModified"].as_str().unwrap().parse().unwrap();
        assert_eq!(wire_timestamp, timestamp);
        assert_eq!(value["priority"], 0.5);
        assert_eq!(serde_json::from_value::<Annotations>(value).unwrap(), annotations);
    }
}

#[test]
fn incoming_annotation_strings_remain_verbatim() {
    for text in ["2025-01-12", "2025-01-12T15:00:58Z", "not-a-date"] {
        let annotations: Annotations =
            serde_json::from_value(json!({"lastModified": text})).unwrap();
        assert_eq!(annotations.last_modified.as_deref(), Some(text));
    }
    for value in [json!({}), json!({"lastModified": null})] {
        assert_eq!(serde_json::from_value::<Annotations>(value).unwrap().last_modified, None);
    }
}

#[test]
fn current_annotation_timestamp_is_between_clock_samples() {
    let before = Timestamp::now();
    let annotations = Annotations::default().with_timestamp_now();
    let after = Timestamp::now();
    let actual: Timestamp = annotations.last_modified.unwrap().parse().unwrap();
    assert!(before <= actual && actual <= after);
}
