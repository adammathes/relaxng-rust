// Stress tests for the RELAX NG validator.
//
// These tests programmatically generate schemas and XML documents to test
// correctness at scale and performance. All tests are self-contained —
// no external schema files needed.
//
// Run with: cargo test --test stress
// Run the larger sizes: cargo test --test stress -- --ignored

use relaxng_model::{Compiler, FsFiles, Syntax};
use relaxng_validator::Validator;
use std::fmt::Write;
use std::time::Instant;
use xmlparser::Tokenizer;

fn validate_generated(schema_xml: &str, doc_xml: &str) -> Result<(), String> {
    let dir = tempfile::tempdir().expect("create temp dir");
    let schema_path = dir.path().join("schema.rng");
    std::fs::write(&schema_path, schema_xml).expect("write schema");

    let mut compiler = Compiler::new(FsFiles, Syntax::Xml);
    let model = compiler
        .compile(&schema_path)
        .map_err(|e| format!("schema compile error: {:?}", e))?;

    let tokenizer = Tokenizer::from(doc_xml);
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

// ══════════════════════════════════════════════════════════════════════════════
//  Interleave Stress Test
// ══════════════════════════════════════════════════════════════════════════════
//
//  Creates a schema with `n` optional elements in an interleave pattern, then
//  generates a document with those elements in reverse order. This directly
//  stresses the interleave derivative computation which historically caused
//  exponential blowup.

fn gen_interleave_schema(n: usize) -> String {
    let mut rng = String::with_capacity(n * 100);
    rng.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<grammar xmlns="http://relaxng.org/ns/structure/1.0"
         datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
  <start>
    <element name="root">
      <interleave>
"#);
    for i in 0..n {
        write!(
            rng,
            "        <optional><element name=\"e{i}\"><text/></element></optional>\n"
        )
        .unwrap();
    }
    rng.push_str(
        r#"      </interleave>
    </element>
  </start>
</grammar>
"#,
    );
    rng
}

fn gen_interleave_doc(n: usize) -> String {
    let mut xml = String::with_capacity(n * 50);
    xml.push_str("<?xml version=\"1.0\"?>\n<root>\n");
    // Elements in reverse order to maximize interleave work
    for i in (0..n).rev() {
        write!(xml, "  <e{i}>value {i}</e{i}>\n").unwrap();
    }
    xml.push_str("</root>\n");
    xml
}

fn validate_interleave(n: usize) {
    let schema_xml = gen_interleave_schema(n);
    let doc_xml = gen_interleave_doc(n);

    let dir = tempfile::tempdir().expect("create temp dir");
    let schema_path = dir.path().join("interleave.rng");
    std::fs::write(&schema_path, &schema_xml).expect("write schema");

    let mut compiler = Compiler::new(FsFiles, Syntax::Xml);
    let model = compiler
        .compile(&schema_path)
        .map_err(|e| format!("schema compile error: {:?}", e))
        .expect("compile interleave schema");

    let start = Instant::now();
    let tokenizer = Tokenizer::from(doc_xml.as_str());
    let mut v = Validator::new(model, tokenizer);
    loop {
        match v.validate_next() {
            Some(Ok(())) => {}
            Some(Err(e)) => panic!("interleave-{n} validation error: {:?}", e),
            None => break,
        }
    }
    let elapsed = start.elapsed();
    eprintln!(
        "  [stress] interleave-{n} — {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[test]
fn interleave_stress_10() {
    validate_interleave(10);
}

#[test]
fn interleave_stress_20() {
    validate_interleave(20);
}

#[test]
fn interleave_stress_30() {
    validate_interleave(30);
}

#[test]
#[ignore] // May be slow depending on interleave optimization
fn interleave_stress_50() {
    validate_interleave(50);
}

// ══════════════════════════════════════════════════════════════════════════════
//  Wide attributes stress test
// ══════════════════════════════════════════════════════════════════════════════
//
//  Stresses att_deriv with many required + optional attributes on a single
//  element. The interleave tests above cover child-element interleave; this
//  covers the attribute-handling path specifically.

fn gen_wide_attr_schema(n_required: usize, n_optional: usize) -> String {
    let mut rng = String::with_capacity((n_required + n_optional) * 80);
    rng.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<grammar xmlns="http://relaxng.org/ns/structure/1.0"
         datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
  <start>
    <element name="root">
"#);
    for i in 0..n_required {
        write!(
            rng,
            "      <attribute name=\"req{i}\"><data type=\"string\"/></attribute>\n"
        )
        .unwrap();
    }
    for i in 0..n_optional {
        write!(
            rng,
            "      <optional><attribute name=\"opt{i}\"><data type=\"string\"/></attribute></optional>\n"
        )
        .unwrap();
    }
    rng.push_str(
        r#"      <empty/>
    </element>
  </start>
</grammar>
"#,
    );
    rng
}

fn gen_wide_attr_doc(n_required: usize, n_optional_present: usize) -> String {
    let mut xml = String::with_capacity((n_required + n_optional_present) * 30);
    xml.push_str("<?xml version=\"1.0\"?>\n<root");
    for i in 0..n_required {
        write!(xml, " req{i}=\"v{i}\"").unwrap();
    }
    for i in 0..n_optional_present {
        write!(xml, " opt{i}=\"v{i}\"").unwrap();
    }
    xml.push_str("/>\n");
    xml
}

#[test]
fn wide_attrs_20_required_20_optional() {
    let schema = gen_wide_attr_schema(20, 20);
    let doc = gen_wide_attr_doc(20, 10); // supply half the optionals
    let start = Instant::now();
    validate_generated(&schema, &doc).expect("valid doc should pass");
    eprintln!(
        "  [stress] wide attrs 20r+10o — {:.2}ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn wide_attrs_50_required() {
    let schema = gen_wide_attr_schema(50, 0);
    let doc = gen_wide_attr_doc(50, 0);
    let start = Instant::now();
    validate_generated(&schema, &doc).expect("valid doc should pass");
    eprintln!(
        "  [stress] wide attrs 50r — {:.2}ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn wide_attrs_missing_required() {
    let schema = gen_wide_attr_schema(10, 0);
    // Only supply 9 of 10 required attributes
    let mut doc = String::from("<?xml version=\"1.0\"?>\n<root");
    for i in 0..9 {
        write!(doc, " req{i}=\"v\"").unwrap();
    }
    doc.push_str("/>\n");
    let result = validate_generated(&schema, &doc);
    assert!(result.is_err(), "missing required attr should fail");
}

// ══════════════════════════════════════════════════════════════════════════════
//  Deep nesting stress test
// ══════════════════════════════════════════════════════════════════════════════
//
//  Pure depth test decoupled from any real-world schema complexity.
//  Tests the element stack and recursive derivative computation.

fn gen_deep_nesting_schema(depth: usize) -> String {
    let mut rng = String::with_capacity(depth * 120);
    rng.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start>
"#);
    for i in 0..depth {
        write!(rng, "    <element name=\"e{i}\">\n").unwrap();
    }
    rng.push_str("    <text/>\n");
    for _ in 0..depth {
        rng.push_str("    </element>\n");
    }
    rng.push_str(
        r#"  </start>
</grammar>
"#,
    );
    rng
}

fn gen_deep_nesting_doc(depth: usize) -> String {
    let mut xml = String::with_capacity(depth * 20);
    xml.push_str("<?xml version=\"1.0\"?>\n");
    for i in 0..depth {
        write!(xml, "<e{i}>").unwrap();
    }
    xml.push_str("leaf");
    for i in (0..depth).rev() {
        write!(xml, "</e{i}>").unwrap();
    }
    xml.push('\n');
    xml
}

#[test]
fn deep_nesting_100() {
    let schema = gen_deep_nesting_schema(100);
    let doc = gen_deep_nesting_doc(100);
    let start = Instant::now();
    validate_generated(&schema, &doc).expect("valid nested doc");
    eprintln!(
        "  [stress] deep nesting 100 — {:.2}ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
#[ignore] // Stack overflow in recursive schema compilation — needs iterative rewrite
fn deep_nesting_500() {
    let schema = gen_deep_nesting_schema(500);
    let doc = gen_deep_nesting_doc(500);
    let start = Instant::now();
    validate_generated(&schema, &doc).expect("valid nested doc");
    eprintln!(
        "  [stress] deep nesting 500 — {:.2}ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn deep_nesting_wrong_leaf() {
    // Valid structure but wrong innermost element name
    let schema = gen_deep_nesting_schema(50);
    let mut doc = String::from("<?xml version=\"1.0\"?>\n");
    for i in 0..49 {
        write!(doc, "<e{i}>").unwrap();
    }
    doc.push_str("<wrong>leaf</wrong>");
    for i in (0..49).rev() {
        write!(doc, "</e{i}>").unwrap();
    }
    let result = validate_generated(&schema, &doc);
    assert!(result.is_err(), "wrong element name at depth should fail");
}

// ══════════════════════════════════════════════════════════════════════════════
//  Choice branching stress test
// ══════════════════════════════════════════════════════════════════════════════
//
//  Schema with N alternative child element types in a choice, then a document
//  that picks different branches. Tests choice derivative computation.

fn gen_choice_schema(n_branches: usize) -> String {
    let mut rng = String::with_capacity(n_branches * 80);
    rng.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start>
    <element name="root">
      <oneOrMore>
        <choice>
"#);
    for i in 0..n_branches {
        write!(
            rng,
            "          <element name=\"branch{i}\"><text/></element>\n"
        )
        .unwrap();
    }
    rng.push_str(
        r#"        </choice>
      </oneOrMore>
    </element>
  </start>
</grammar>
"#,
    );
    rng
}

fn gen_choice_doc(n_branches: usize, n_children: usize) -> String {
    let mut xml = String::with_capacity(n_children * 40);
    xml.push_str("<?xml version=\"1.0\"?>\n<root>\n");
    for i in 0..n_children {
        let branch = i % n_branches;
        write!(xml, "  <branch{branch}>text</branch{branch}>\n").unwrap();
    }
    xml.push_str("</root>\n");
    xml
}

#[test]
fn choice_50_branches_200_children() {
    let schema = gen_choice_schema(50);
    let doc = gen_choice_doc(50, 200);
    let start = Instant::now();
    validate_generated(&schema, &doc).expect("valid choice doc");
    eprintln!(
        "  [stress] choice 50 branches x 200 children — {:.2}ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn choice_100_branches_500_children() {
    let schema = gen_choice_schema(100);
    let doc = gen_choice_doc(100, 500);
    let start = Instant::now();
    validate_generated(&schema, &doc).expect("valid choice doc");
    eprintln!(
        "  [stress] choice 100 branches x 500 children — {:.2}ms",
        start.elapsed().as_secs_f64() * 1000.0
    );
}

#[test]
fn choice_wrong_branch_name() {
    let schema = gen_choice_schema(10);
    let mut doc = gen_choice_doc(10, 5);
    // Inject an element not in any branch
    let pos = doc.rfind("</root>").unwrap();
    doc.insert_str(pos, "  <nonexistent>x</nonexistent>\n");
    let result = validate_generated(&schema, &doc);
    assert!(result.is_err(), "element outside choice branches should fail");
}
