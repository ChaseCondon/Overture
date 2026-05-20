//! Negative tests: validation must catch the structural problems it claims to.

use stardust_patch::{PatchDocument, ValidationError};

const BASE: &str = include_str!("fixtures/casual.json");

fn mutate(f: impl FnOnce(&mut serde_json::Value)) -> Vec<ValidationError> {
    let mut v: serde_json::Value = serde_json::from_str(BASE).unwrap();
    f(&mut v);
    let s = v.to_string();
    let doc: PatchDocument = PatchDocument::from_json(&s).expect("parses");
    doc.graph
        .validate()
        .expect_err("expected validation errors")
}

#[test]
fn detects_unknown_wire_endpoint() {
    let errs = mutate(|v| {
        v["graph"]["wires"][0]["toNode"] = "nope".into();
    });
    assert!(
        errs.iter()
            .any(|e| matches!(e, ValidationError::WireUnknownEndpoint { .. })),
        "got: {errs:#?}"
    );
}

#[test]
fn detects_unknown_port() {
    let errs = mutate(|v| {
        v["graph"]["wires"][0]["toPort"] = "missing-port".into();
    });
    assert!(
        errs.iter()
            .any(|e| matches!(e, ValidationError::WireUnknownPort { .. })),
        "got: {errs:#?}"
    );
}

#[test]
fn detects_signal_mismatch() {
    // Wire MIDI-out -> audio-in (n1.out -> n3.in-l).
    let errs = mutate(|v| {
        v["graph"]["wires"][0]["toNode"] = "n3".into();
        v["graph"]["wires"][0]["toPort"] = "in-l".into();
    });
    assert!(
        errs.iter()
            .any(|e| matches!(e, ValidationError::WireSignalMismatch { .. })),
        "got: {errs:#?}"
    );
}

#[test]
fn detects_direction_violation() {
    // Try to wire an "in" port as the source.
    let errs = mutate(|v| {
        v["graph"]["wires"][0]["fromNode"] = "n3".into();
        v["graph"]["wires"][0]["fromPort"] = "in-l".into();
        v["graph"]["wires"][0]["toNode"] = "n3".into();
        v["graph"]["wires"][0]["toPort"] = "in-r".into();
    });
    assert!(
        errs.iter()
            .any(|e| matches!(e, ValidationError::WireDirection { .. })),
        "got: {errs:#?}"
    );
}

#[test]
fn detects_duplicate_node_id() {
    let errs = mutate(|v| {
        // Clone n1 so two nodes share id "n1".
        let dup = v["graph"]["nodes"][0].clone();
        v["graph"]["nodes"].as_array_mut().unwrap().push(dup);
    });
    assert!(
        errs.iter()
            .any(|e| matches!(e, ValidationError::DuplicateNodeId(_))),
        "got: {errs:#?}"
    );
}

#[test]
fn rejects_wrong_kind() {
    let mut v: serde_json::Value = serde_json::from_str(BASE).unwrap();
    v["kind"] = "stardust.show".into();
    let err = PatchDocument::from_json(&v.to_string()).expect_err("should reject");
    assert!(format!("{err}").contains("stardust.show"));
}

#[test]
fn rejects_newer_schema() {
    let mut v: serde_json::Value = serde_json::from_str(BASE).unwrap();
    v["schemaVersion"] = 9999.into();
    let err = PatchDocument::from_json(&v.to_string()).expect_err("should reject");
    assert!(format!("{err}").contains("v9999"));
}
