The `spectest.xml` file is taken from the
[jing-trang project](https://github.com/relaxng/jing-trang/tree/master/mod/rng-validate/test), and
`spectest.rs` is a test-driver which executes the test cases defined in the XML.

## Real-world schema tests

`real_world.rs` validates documents against real schemas downloaded from
official sources (W3C, OASIS, IETF).  The schemas are **not** checked in.
Run the download script first:

```
./testdata/download.sh
cargo test --test real_world
```

| Schema | Source | Status |
|--------|--------|--------|
| Atom 1.0 | RFC 4287 (gist conversion) | Compiles and validates |
| SVG 1.1 | W3C (modular, ~40 files) | Compiles and validates |
| DocBook 5.0 | OASIS (single file, ~15k lines) | Known bug: validator panic on placeholder resolution |
| XHTML 1.1 | W3C (modular, ~25 files) | Known bug: datatypeLibrary not propagated across includes |