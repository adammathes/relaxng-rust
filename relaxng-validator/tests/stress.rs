// Stress tests for the RELAX NG validator.
//
// These tests programmatically generate large XML documents and validate them
// against real-world schemas, testing both correctness at scale and performance.
//
// Run with: cargo test --test stress
// Run the larger sizes: cargo test --test stress -- --ignored

use relaxng_model::{Compiler, FsFiles, Syntax};
use relaxng_validator::Validator;
use std::fmt::Write;
use std::path::Path;
use std::time::Instant;
use xmlparser::Tokenizer;

/// Compile `schema` and validate `doc` XML string.
fn validate_str(schema_path: &str, doc: &str) -> Result<(), String> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("testdata/real-world");

    let schema_path = base.join(schema_path);

    let mut compiler = Compiler::new(FsFiles, Syntax::Xml);
    let model = compiler
        .compile(&schema_path)
        .map_err(|e| format!("schema compile error: {:?}", e))?;

    let tokenizer = Tokenizer::from(doc);
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

/// Helper: validates and prints timing information.
fn validate_timed(label: &str, schema: &str, doc: &str) {
    let start = Instant::now();
    let result = validate_str(schema, doc);
    let elapsed = start.elapsed();
    eprintln!(
        "  [stress] {} — {} elements, {:.2}ms — {}",
        label,
        doc.matches('<').count(), // rough element count
        elapsed.as_secs_f64() * 1000.0,
        if result.is_ok() { "PASS" } else { "FAIL" }
    );
    result.expect(label);
}

// ══════════════════════════════════════════════════════════════════════════════
//  Atom Feed Generator
// ══════════════════════════════════════════════════════════════════════════════

fn gen_atom_feed(n_entries: usize) -> String {
    let mut xml = String::with_capacity(n_entries * 300);
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Stress Test Feed</title>
  <id>urn:uuid:stress-test-feed</id>
  <updated>2026-01-01T00:00:00Z</updated>
  <link href="https://example.com/feed" rel="self" type="application/atom+xml"/>
  <link href="https://example.com/" rel="alternate"/>
  <author>
    <name>Stress Tester</name>
    <uri>https://example.com/tester</uri>
    <email>tester@example.com</email>
  </author>
  <generator uri="https://example.com/gen" version="1.0">StressGen</generator>
  <subtitle type="text">A feed with many entries for stress testing</subtitle>
"#);

    for i in 0..n_entries {
        let day = (i % 28) + 1;
        let hour = i % 24;
        let min = i % 60;
        write!(
            xml,
            r#"  <entry>
    <title>Entry {i}: A test entry with a somewhat long title for testing</title>
    <id>urn:uuid:entry-{i:06}</id>
    <updated>2026-01-{day:02}T{hour:02}:{min:02}:00Z</updated>
    <published>2025-12-{day:02}T{hour:02}:{min:02}:00Z</published>
    <link href="https://example.com/entries/{i}" rel="alternate"/>
    <category term="test" scheme="https://example.com/categories" label="Test"/>
    <summary type="text">Summary for entry number {i}.</summary>
    <content type="text">This is the full content of entry number {i}. It contains some text to make the document larger and exercise the validator with repeated elements.</content>
    <author>
      <name>Author {author}</name>
    </author>
  </entry>
"#,
            author = i % 5
        )
        .unwrap();
    }

    xml.push_str("</feed>\n");
    xml
}

#[test]
#[ignore] // slow due to interleave patterns in atom schema (~60s for 100 entries)
fn atom_stress_100() {
    let doc = gen_atom_feed(100);
    validate_timed("atom 100 entries", "atom.rng", &doc);
}

#[test]
#[ignore] // ~5 minutes due to interleave patterns in atom schema
fn atom_stress_500() {
    let doc = gen_atom_feed(500);
    validate_timed("atom 500 entries", "atom.rng", &doc);
}

#[test]
#[ignore] // Takes ~5-10 seconds
fn atom_stress_2000() {
    let doc = gen_atom_feed(2000);
    validate_timed("atom 2000 entries", "atom.rng", &doc);
}

// ══════════════════════════════════════════════════════════════════════════════
//  XHTML Page Generator
// ══════════════════════════════════════════════════════════════════════════════

fn gen_xhtml_page(n_sections: usize, table_rows: usize) -> String {
    let mut xml = String::with_capacity(n_sections * 500 + table_rows * 200);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en">
<head>
  <title>XHTML Stress Test Page</title>
  <meta name="generator" content="stress-test"/>
  <meta name="description" content="A page with many sections and a large table"/>
  <link rel="stylesheet" type="text/css" href="style.css"/>
  <style type="text/css">body { font-family: sans-serif; }</style>
</head>
<body>
  <h1>XHTML Stress Test</h1>
"#);

    // Generate sections with headings, paragraphs, and inline elements
    for i in 0..n_sections {
        let h_level = (i % 5) + 2; // h2 through h6
        write!(
            xml,
            r#"
  <h{h_level} id="sec-{i}" class="section-heading">Section {i}: A test heading</h{h_level}>
  <p class="intro" style="margin: 1em">This is paragraph one of section {i}. It contains
  <em>emphasized text</em>, <strong>strong text</strong>, <code>code</code>,
  <abbr title="abbreviation">abbr</abbr>, and <a href="https://example.com/s/{i}">a link</a>.
  Also <sub>subscript</sub> and <sup>superscript</sup> and <kbd>keyboard</kbd> input.</p>
  <p>Second paragraph with <span class="highlight">a span</span>, <tt>teletype</tt>,
  <i>italic</i>, <b>bold</b>, <big>big</big>, <small>small</small>,
  <cite>citation</cite>, <dfn>definition</dfn>, <samp>sample</samp>,
  <var>variable</var>, and <q cite="https://example.com">a quotation</q>.</p>
"#
        )
        .unwrap();

        // Every 10th section gets a list
        if i % 10 == 0 {
            xml.push_str("  <ul>\n");
            for j in 0..5 {
                write!(
                    xml,
                    "    <li>List item {j} in section {i} with <strong>bold</strong> text.</li>\n"
                )
                .unwrap();
            }
            xml.push_str("  </ul>\n");
        }

        // Every 15th section gets a definition list
        if i % 15 == 0 {
            xml.push_str("  <dl>\n");
            for j in 0..3 {
                write!(
                    xml,
                    "    <dt><dfn>Term {j}</dfn></dt>\n    <dd>Definition for term {j} in section {i}.</dd>\n"
                )
                .unwrap();
            }
            xml.push_str("  </dl>\n");
        }
    }

    // Generate a large table
    if table_rows > 0 {
        xml.push_str(
            r#"
  <table summary="Large test table" border="1" cellspacing="0" cellpadding="3">
    <caption>Stress Test Data Table</caption>
    <thead>
      <tr>
        <th scope="col" id="c1">Row #</th>
        <th scope="col" id="c2">Name</th>
        <th scope="col" id="c3">Value</th>
        <th scope="col" id="c4">Status</th>
      </tr>
    </thead>
    <tbody>
"#,
        );
        for r in 0..table_rows {
            write!(
                xml,
                r#"      <tr>
        <td>{r}</td>
        <td>Item <em>{r}</em></td>
        <td><code>{val}</code></td>
        <td>{status}</td>
      </tr>
"#,
                val = r * 17 + 42,
                status = if r % 3 == 0 { "Active" } else { "Pending" }
            )
            .unwrap();
        }
        xml.push_str("    </tbody>\n  </table>\n");
    }

    xml.push_str("</body>\n</html>\n");
    xml
}

#[test]
fn xhtml_stress_100_50() {
    let doc = gen_xhtml_page(100, 50);
    validate_timed("xhtml 100 sections + 50 rows", "xhtml1-strict.rng", &doc);
}

#[test]
fn xhtml_stress_500_200() {
    let doc = gen_xhtml_page(500, 200);
    validate_timed("xhtml 500 sections + 200 rows", "xhtml1-strict.rng", &doc);
}

#[test]
#[ignore] // Larger test
fn xhtml_stress_1000_500() {
    let doc = gen_xhtml_page(1000, 500);
    validate_timed("xhtml 1000 sections + 500 rows", "xhtml1-strict.rng", &doc);
}

// Also test against the full XHTML schema
#[test]
fn xhtml_full_stress_100_50() {
    let doc = gen_xhtml_page(100, 50);
    validate_timed(
        "xhtml-full 100 sections + 50 rows",
        "xhtml1-strict-full.rng",
        &doc,
    );
}

// ══════════════════════════════════════════════════════════════════════════════
//  DocBook Book Generator
// ══════════════════════════════════════════════════════════════════════════════

fn gen_docbook_book(n_chapters: usize, section_depth: usize) -> String {
    let mut xml = String::with_capacity(n_chapters * section_depth * 500);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<book xmlns="http://docbook.org/ns/docbook"
      xmlns:xlink="http://www.w3.org/1999/xlink"
      version="5.0" xml:lang="en">
  <info>
    <title>Stress Test Book</title>
    <author>
      <personname>
        <firstname>Stress</firstname>
        <surname>Tester</surname>
      </personname>
    </author>
    <copyright>
      <year>2026</year>
      <holder>Test Corp</holder>
    </copyright>
    <abstract>
      <para>A book with many chapters and deeply nested sections.</para>
    </abstract>
  </info>
"#);

    for ch in 0..n_chapters {
        write!(
            xml,
            r#"
  <chapter xml:id="ch-{ch}">
    <title>Chapter {ch}: Testing Deep Structure</title>
    <para>Introduction to chapter {ch}. This chapter exercises <emphasis>deep nesting</emphasis>
    of section elements, <link xlink:href="https://example.com">links</link>,
    and various inline elements like <code>code</code>, <literal>literal</literal>,
    <command>command</command>, <filename>/etc/config</filename>, and
    <varname>variable_name</varname>.</para>
"#
        )
        .unwrap();

        // Generate nested sections
        gen_docbook_section(&mut xml, ch, 0, section_depth);

        xml.push_str("  </chapter>\n");
    }

    xml.push_str("</book>\n");
    xml
}

fn gen_docbook_section(xml: &mut String, chapter: usize, depth: usize, max_depth: usize) {
    let indent = "    ".repeat(depth + 2);
    let id = format!("ch{chapter}-s{depth}");
    write!(
        xml,
        r#"{indent}<section xml:id="{id}">
{indent}  <title>Section at depth {depth}</title>
{indent}  <para>Content at depth {depth} in chapter {chapter}. This section contains
{indent}  <emphasis>emphasized</emphasis> text, <classname>ClassName</classname>,
{indent}  <methodname>method_name</methodname>, and <parameter>param</parameter>.</para>
{indent}  <programlisting language="rust">fn level_{depth}() {{
{indent}      // depth {depth} code
{indent}  }}</programlisting>
"#
    )
    .unwrap();

    // Add a list every other depth level
    if depth % 2 == 0 {
        write!(
            xml,
            r#"{indent}  <itemizedlist>
{indent}    <listitem><para>Item A at depth {depth}.</para></listitem>
{indent}    <listitem><para>Item B at depth {depth}.</para></listitem>
{indent}  </itemizedlist>
"#
        )
        .unwrap();
    }

    // Recurse deeper
    if depth < max_depth {
        gen_docbook_section(xml, chapter, depth + 1, max_depth);
    }

    write!(xml, "{indent}</section>\n").unwrap();
}

#[test]
fn docbook_stress_10ch_5deep() {
    let doc = gen_docbook_book(10, 5);
    validate_timed(
        "docbook 10 chapters x 5 deep",
        "docbook5-subset.rng",
        &doc,
    );
}

#[test]
fn docbook_stress_50ch_10deep() {
    let doc = gen_docbook_book(50, 10);
    validate_timed(
        "docbook 50 chapters x 10 deep",
        "docbook5-subset.rng",
        &doc,
    );
}

#[test]
#[ignore] // Takes longer
fn docbook_stress_100ch_15deep() {
    let doc = gen_docbook_book(100, 15);
    validate_timed(
        "docbook 100 chapters x 15 deep",
        "docbook5-subset.rng",
        &doc,
    );
}

// Also test against the full DocBook schema
#[test]
fn docbook_full_stress_10ch_5deep() {
    let doc = gen_docbook_book(10, 5);
    validate_timed(
        "docbook-full 10 chapters x 5 deep",
        "docbook5-full.rng",
        &doc,
    );
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

    // Write schema to a temp file
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
//  SVG Document Generator
// ══════════════════════════════════════════════════════════════════════════════

fn gen_svg_doc(n_shapes: usize) -> String {
    let mut xml = String::with_capacity(n_shapes * 200);
    xml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg"
     xmlns:xlink="http://www.w3.org/1999/xlink"
     viewBox="0 0 1000 1000" width="1000" height="1000">
  <title>Generated SVG stress test</title>
  <defs>
    <linearGradient id="g1" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="red"/>
      <stop offset="100%" stop-color="blue"/>
    </linearGradient>
  </defs>
"#,
    );

    for i in 0..n_shapes {
        let x = (i * 7) % 900;
        let y = (i * 13) % 900;
        match i % 6 {
            0 => write!(
                xml,
                "  <rect x=\"{x}\" y=\"{y}\" width=\"50\" height=\"30\" fill=\"#{:06x}\" stroke=\"black\" stroke-width=\"1\" opacity=\"0.8\"/>\n",
                (i * 12345) % 0xFFFFFF
            ).unwrap(),
            1 => write!(
                xml,
                "  <circle cx=\"{x}\" cy=\"{y}\" r=\"20\" fill=\"url(#g1)\" stroke=\"gray\"/>\n"
            ).unwrap(),
            2 => write!(
                xml,
                "  <ellipse cx=\"{x}\" cy=\"{y}\" rx=\"30\" ry=\"15\" fill=\"green\" fill-opacity=\"0.5\"/>\n"
            ).unwrap(),
            3 => write!(
                xml,
                "  <line x1=\"{x}\" y1=\"{y}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" stroke-width=\"2\"/>\n",
                x + 50, y + 30
            ).unwrap(),
            4 => write!(
                xml,
                "  <path d=\"M {x} {y} L {} {} L {} {} Z\" fill=\"purple\" stroke=\"none\"/>\n",
                x + 40, y, x + 20, y + 35
            ).unwrap(),
            _ => write!(
                xml,
                "  <text x=\"{x}\" y=\"{y}\" font-size=\"12\" fill=\"black\">Text {i}</text>\n"
            ).unwrap(),
        }
    }

    xml.push_str("</svg>\n");
    xml
}

#[test]
fn svg_stress_100() {
    let doc = gen_svg_doc(100);
    validate_timed("svg 100 shapes", "svg11.rng", &doc);
}

#[test]
fn svg_stress_500() {
    let doc = gen_svg_doc(500);
    validate_timed("svg 500 shapes", "svg11.rng", &doc);
}

#[test]
#[ignore]
fn svg_stress_2000() {
    let doc = gen_svg_doc(2000);
    validate_timed("svg 2000 shapes", "svg11.rng", &doc);
}

// ══════════════════════════════════════════════════════════════════════════════
//  Error-at-end tests
// ══════════════════════════════════════════════════════════════════════════════
//
//  Generate a large valid document, then inject a single error near the end.
//  This verifies the validator doesn't lose state during long runs.

#[test]
#[ignore] // slow due to interleave patterns in atom schema (~60s for 200 entries)
fn atom_error_at_end() {
    // Generate a large valid feed, then add an entry with invalid dateTime
    let mut doc = gen_atom_feed(200);
    // Remove the closing </feed> and add a bad entry
    doc.truncate(doc.rfind("</feed>").unwrap());
    doc.push_str(
        r#"  <entry>
    <title>Bad Entry</title>
    <id>urn:uuid:bad-entry</id>
    <updated>NOT-A-DATE</updated>
  </entry>
</feed>
"#,
    );
    let result = validate_str("atom.rng", &doc);
    assert!(
        result.is_err(),
        "should detect invalid dateTime after 200 valid entries"
    );
}

#[test]
fn xhtml_error_at_end() {
    // Generate a large valid page, then add an img missing alt
    let mut doc = gen_xhtml_page(100, 0);
    // Inject an invalid element before </body>
    let insert_pos = doc.rfind("</body>").unwrap();
    doc.insert_str(
        insert_pos,
        "<p><img src=\"bad.png\"/></p>\n", // missing required alt attribute
    );
    let result = validate_str("xhtml1-strict.rng", &doc);
    assert!(
        result.is_err(),
        "should detect missing alt after 100 valid sections"
    );
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
