// Integration tests for EPUB RELAX NG schema validation.
//
// These tests validate real EPUB components (container.xml, OPF package,
// XHTML content) against the official EPUB 3 RELAX NG schemas from epubcheck.
//
// Run with: cargo test --test epub

use relaxng_model::{Compiler, FsFiles, Syntax};
use relaxng_validator::Validator;
use std::path::Path;
use xmlparser::Tokenizer;

fn validate(schema_path: &str, xml_path: &str) -> Result<(), String> {
    let schema = Path::new(schema_path);
    let mut compiler = Compiler::new(FsFiles, Syntax::Compact);
    let model = compiler
        .compile(schema)
        .map_err(|e| format!("schema compile error: {:?}", e))?;

    let xml = std::fs::read_to_string(xml_path)
        .map_err(|e| format!("failed to read {}: {}", xml_path, e))?;
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

#[test]
fn epub_ocf_container_validation() {
    validate(
        "tests/epub_schemas/ocf-container-30.rnc",
        "tests/epub_testdata/container.xml",
    )
    .expect("OCF container.xml should validate against ocf-container-30.rnc");
}

#[test]
fn epub_opf_package_validation() {
    validate(
        "tests/epub_schemas/package-30.rnc",
        "tests/epub_testdata/content.opf",
    )
    .expect("OPF package should validate against package-30.rnc");
}

#[test]
fn epub_xhtml_toc_validation() {
    validate(
        "tests/epub_schemas/epub-xhtml-30.rnc",
        "tests/epub_testdata/toc.xhtml",
    )
    .expect("EPUB XHTML toc should validate against epub-xhtml-30.rnc");
}

#[test]
fn epub_xhtml_content_validation() {
    validate(
        "tests/epub_schemas/epub-xhtml-30.rnc",
        "tests/epub_testdata/content.xhtml",
    )
    .expect("EPUB XHTML content should validate against epub-xhtml-30.rnc");
}
