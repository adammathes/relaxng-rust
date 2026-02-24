// Integration tests against real-world schemas and documents.
//
// Each test case validates a (schema, document) pair and checks the expected
// outcome (valid or invalid).  Test data lives in `testdata/real-world/`.

use relaxng_model::{Compiler, FsFiles, Syntax};
use relaxng_validator::Validator;
use std::path::Path;
use xmlparser::Tokenizer;

/// Compile `schema` and validate `doc` (both relative to `testdata/real-world/`).
/// Returns `Ok(())` on successful validation, `Err(msg)` on any error.
fn validate(schema: &str, doc: &str) -> Result<(), String> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("testdata/real-world");

    let schema_path = base.join(schema);
    let doc_path = base.join(doc);

    let mut compiler = Compiler::new(FsFiles, Syntax::Xml);
    let model = compiler
        .compile(&schema_path)
        .map_err(|e| format!("schema compile error: {:?}", e))?;

    let xml = std::fs::read_to_string(&doc_path)
        .map_err(|e| format!("could not read {}: {}", doc_path.display(), e))?;

    let tokenizer = Tokenizer::from(xml.as_str());
    let mut v = Validator::new(model, tokenizer);
    loop {
        match v.validate_next() {
            Some(Ok(())) => {}
            Some(Err(e)) => return Err(format!("{:?}", e)),
            None => break,
        }
    }
    Ok(())
}

// ── Atom 1.0 ──────────────────────────────────────────────────────────────────

#[test]
fn atom_valid_entry() {
    validate("atom.rng", "atom-valid-entry.xml").expect("should be valid");
}

#[test]
fn atom_invalid_missing_id() {
    assert!(
        validate("atom.rng", "atom-invalid-missing-id.xml").is_err(),
        "should be invalid: missing <id> element"
    );
}

#[test]
fn atom_invalid_bad_datetime() {
    assert!(
        validate("atom.rng", "atom-invalid-bad-datetime.xml").is_err(),
        "should be invalid: malformed xsd:dateTime"
    );
}

#[test]
fn atom_valid_minimal() {
    validate("atom.rng", "atom-valid-minimal.xml").expect("should be valid");
}

#[test]
fn atom_valid_full() {
    validate("atom.rng", "atom-valid-full.xml").expect("should be valid");
}

#[test]
fn atom_valid_xhtml_content() {
    validate("atom.rng", "atom-valid-xhtml-content.xml").expect("should be valid");
}

// ── XHTML 1.0 Strict ──────────────────────────────────────────────────────────

#[test]
fn xhtml_valid_simple() {
    validate("xhtml1-strict.rng", "xhtml-valid-simple.xml").expect("should be valid");
}

#[test]
fn xhtml_valid_table() {
    // Exercises NMTOKEN/NMTOKENS (Bug #3 regression guard)
    validate("xhtml1-strict.rng", "xhtml-valid-table.xml").expect("should be valid");
}

#[test]
fn xhtml_valid_ids() {
    // Exercises xml:id / ID attributes (Bug #3 regression guard)
    validate("xhtml1-strict.rng", "xhtml-valid-ids.xml").expect("should be valid");
}

#[test]
fn xhtml_invalid_missing_alt() {
    assert!(
        validate("xhtml1-strict.rng", "xhtml-invalid-missing-alt.xml").is_err(),
        "should be invalid: <img> missing required alt attribute"
    );
}

// ── DocBook 5 subset ──────────────────────────────────────────────────────────

#[test]
fn docbook_valid_article() {
    validate("docbook5-subset.rng", "docbook-valid-article.xml").expect("should be valid");
}

#[test]
fn docbook_valid_book() {
    validate("docbook5-subset.rng", "docbook-valid-book.xml").expect("should be valid");
}

#[test]
fn docbook_invalid_bad_gyear() {
    // Exercises gYear datatype validation (Bug #4 regression guard)
    assert!(
        validate("docbook5-subset.rng", "docbook-invalid-bad-gYear.xml").is_err(),
        "should be invalid: malformed xsd:gYear value"
    );
}

// ── XSD datatypes ─────────────────────────────────────────────────────────────

#[test]
fn xsd_valid_all_types() {
    // Exercises all 17 datatypes added in Bug #4 fix, plus pattern/positiveInteger fixes
    validate("xsd-datatypes.rng", "xsd-valid-all-types.xml").expect("should be valid");
}

#[test]
fn xsd_invalid_types() {
    assert!(
        validate("xsd-datatypes.rng", "xsd-invalid-types.xml").is_err(),
        "should be invalid: various XSD type constraint violations"
    );
}
