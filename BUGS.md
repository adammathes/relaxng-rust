# Phase 5 Bug Report: Real-World Schema Testing

**Methodology:** Each schema/document pair was validated with both Jing (reference Java
implementation, the ground truth oracle) and our Rust tool. Divergences indicate bugs.
Test corpus created in `testdata/real-world/`.

---

## Summary Table

| # | Severity | Category | Status |
|---|----------|----------|--------|
| B1 | **Critical** | CLI: `.rng` extension not recognized → always uses compact parser | **FIXED** |
| B2 | **High** | Section 7: `Ref` content-type false positive blocks real-world schemas | **FIXED** |
| B3 | **High** | Validator panics on `NMTOKEN`/`NMTOKENS` validation | **FIXED** |
| B4 | **Medium** | 17 XSD datatypes fail at schema-compile time with "Unsupported" | **FIXED** |
| B5 | **Medium** | `positiveInteger` incorrectly accepts 0 | **FIXED** |
| B6 | **Medium** | XSD `pattern` facet matches substrings instead of full value | **FIXED** |
| B7 | **Low** | `anyURI` rejects values with spaces (Jing accepts them) | **FIXED** |

---

## Bug #1 — CLI tool ignores `.rng` extension (FIXED in this branch)

**Severity:** Critical — makes the tool useless for XML-syntax schemas.

**Root cause:** `validate()` in `relaxng-tool/src/main.rs` called `Compiler::default()`,
which hardcodes `Syntax::Compact`. Every `.rng` file was silently fed to the compact
parser, producing a "Parse error at 1:1" diagnostic that spans the entire file.

**Fix applied:** The tool now inspects the file extension:
```rust
let syntax = match schema.extension().and_then(|e| e.to_str()) {
    Some("rng") => Syntax::Xml,
    _ => Syntax::Compact,
};
let mut compiler = Compiler::new(relaxng_model::FsFiles, syntax);
```

**Affected schemas:** Every `.rng` (XML-syntax) schema — Atom, XHTML, DocBook, ODF,
TEI, DITA, and the spectest suite's own `correct.rng`/`incorrect.rng` files (those
are loaded programmatically with `Syntax::Xml`, so they were unaffected).

**Minimal reproducer:**
```xml
<!-- test.rng -->
<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start><element name="r"><text/></element></start>
</grammar>
```
```
$ rng validate test.rng doc.xml   # before fix: Parse error at 1:1
```

---

## Bug #2 — Section 7 "string sequence" false positive for `Ref` patterns (FIXED)

**Severity:** High — blocks compilation of any schema that uses an attribute-group ref
adjacent to a `data`/`value` element content pattern. This affects the Atom 1.0 schema
and the pattern is extremely common in real-world schemas.

**Root cause:** In `relaxng-model/src/restrictions.rs`, the `content_type()` function
unconditionally returns `ContentType::Complex` for `Pattern::Ref`.

**Fix applied:** `content_type()` now follows `Pattern::Ref` recursively to compute the
actual content type. A `seen: HashSet<usize>` set detects cycles (cyclic refs default to
`ContentType::Complex` since cycles typically contain elements). An attribute-only group
now correctly returns `ContentType::Empty`, making it groupable with `data`/`value`
patterns.

---

## Bug #3 — `NMTOKEN` and `NMTOKENS` panic at validation time (FIXED)

**Severity:** High — panics crash the process. Schemas using `NMTOKEN`/`NMTOKENS`
(XHTML class/rel/rev attributes, DocBook role attributes, many others) compile without
error but crash on the first document that exercises those attributes.

**Root cause:** `is_valid()` for `NmTokens`, `NmToken`, and `Token` called
`unimplemented!()`.

**Fix applied:**
- Added `is_name_char()` (NCNameChar + `:`) and `is_valid_nmtoken()`/`is_valid_nmtokens()`
  helpers using the existing `relaxng_syntax::ncname` character classification tables.
- `NmToken`: validates that the entire string is one or more XML NameChars.
- `NmTokens`: validates a whitespace-separated list of one or more NMTOKENs; length
  facets count the number of tokens (not character length).
- `Token`: validates that the value equals its whitespace-collapsed form (no leading/
  trailing/consecutive spaces); length facets check character count.

---

## Bug #4 — 17 XSD datatypes fail at schema compile time (FIXED)

**Severity:** Medium — schemas using these types fail to compile at all.

**Fix applied:** All 17 types are now implemented:

| Datatype | Implementation |
|----------|---------------|
| `time` | Regex: `HH:MM:SS[.frac][tz]` |
| `Name` | XML Name char validation (NCNameStartChar\|`:` + NCNameChar\|`:`)  |
| `nonNegativeInteger` | `BigUint::from_str` (≥ 0 by type) + facets |
| `negativeInteger` | `BigInt::from_str` with `< 0` check + facets |
| `nonPositiveInteger` | `BigInt::from_str` with `≤ 0` check + facets |
| `byte` | `i8::from_str` + facets |
| `unsignedByte` | `u8::from_str` + facets |
| `float` | `f32::from_str` + pattern facet |
| `base64Binary` | Regex `[A-Za-z0-9+/\s]*={0,2}`; length counts decoded octets |
| `hexBinary` | Regex `([0-9A-Fa-f]{2})*`; length counts octets |
| `gYear` | Regex: `-?\d{4,}[tz]?` |
| `gYearMonth` | Regex: `-?\d{4,}-\d{2}[tz]?` |
| `gMonth` | Regex: `--\d{2}[tz]?` |
| `gMonthDay` | Regex: `--\d{2}-\d{2}[tz]?` |
| `gDay` | Regex: `---\d{2}[tz]?` |
| `QName` | Syntax-only validation: `NCName:NCName` or `NCName` |
| `ENTITY` / `ENTITIES` | NCName validation + length facets |

---

## Bug #5 — `positiveInteger` incorrectly accepts 0 (FIXED)

**Severity:** Medium — silent false-positive (accepts invalid documents).

**Root cause:** `positive_integer()` built a `MinMaxFacet<BigUint>` with no lower bound.

**Fix applied:** The `is_valid()` arm for `PositiveInteger` now enforces `v >= 1` as a
hard constraint before applying any user-supplied facets.

---

## Bug #6 — XSD `pattern` facet matches substrings, not full values (FIXED)

**Severity:** Medium — silent false-positive (accepts invalid documents when pattern
constraints are present).

**Root cause:** The regex was compiled unanchored, so it matched anywhere in the string.

**Fix applied:** The `Compiler::pattern()` method now wraps every `pattern` param value
with `^(?:...)$` anchors before compiling the regex, ensuring full-value matching as the
XSD spec requires.

---

## Bug #7 — `anyURI` rejects values with spaces (FIXED)

**Severity:** Low — Jing (following XSD 1.0 practice) accepts any string for `anyURI`.

**Fix applied:** The `AnyURI` `is_valid()` arm now accepts any string (dropping the
`uriparse::URIReference` strictness check), matching Jing's behavior. The optional
`pattern` facet is still applied if present.

---

## Schema / Tool Coverage Results

After fixing all bugs, the following was confirmed to work correctly:

| Schema | Valid docs pass | Invalid docs rejected | Notes |
|--------|----------------|----------------------|-------|
| Atom 1.0 | **Yes** | **Yes** | Bug #2 fixed |
| XHTML 1.0 Strict (simple) | **Yes** | **Yes** (missing alt) | |
| XHTML 1.0 Strict (table) | **Yes** | — | Bug #3 fixed |
| XHTML 1.0 Strict (ids) | **Yes** | — | Bug #3 fixed |
| DocBook 5 (article) | **Yes** | **Yes** | |
| DocBook 5 (book) | **Yes** | **Yes** | |
| XSD datatypes (basic) | **Yes** | **Yes** | |

### What works well
- Mixed content (text + elements interleaved in any order)
- `interleave` with all element permutations
- Required/optional attribute handling
- `anyName`/`nsName` wildcard patterns
- `choice`, `oneOrMore`, `zeroOrMore` element patterns
- `dateTime`, `date`, `integer`, `decimal`, `boolean`, `language`, `anyURI` (mostly)
- Nested `section` recursion up to depth 100+

---

## Prioritization for Pre-Performance Phase

**Recommend fixing before performance work:**
1. **Bug #2** (Ref content-type) — blocks Atom, XHTML, and most real-world schemas from
   even compiling. Nothing else can be tested without this.
2. **Bug #3** (NMTOKEN panic) — crash on XHTML class/rel/name attributes, very common.
3. **Bug #6** (pattern anchoring) — silent false-positive, easy one-line fix.
4. **Bug #5** (positiveInteger=0) — silent false-positive, trivial fix.

**Reasonable to defer until after performance:**
5. **Bug #4** (unsupported datatypes) — graceful errors, not crashes. Large surface area
   but individually straightforward. Priority order within: `float` > `nonNegativeInteger` >
   `gYear` > `base64Binary`/`hexBinary` > the rest.
6. **Bug #7** (anyURI strictness) — low severity, edge case.

---

## Test Files Added

All test files are in `testdata/real-world/`:

### Schemas
- `atom.rng` — Atom 1.0 feed schema (RFC 4287 subset)
- `xhtml1-strict.rng` — XHTML 1.0 Strict (common elements)
- `docbook5-subset.rng` — DocBook 5.x article/book/chapter/section subset
- `xsd-datatypes.rng` — Schema exercising many XSD datatypes

### Documents
- `atom-valid-minimal.xml` / `atom-valid-full.xml` / `atom-valid-xhtml-content.xml` / `atom-valid-entry.xml`
- `atom-invalid-missing-id.xml` / `atom-invalid-bad-datetime.xml`
- `xhtml-valid-simple.xml` / `xhtml-valid-table.xml` / `xhtml-valid-ids.xml`
- `xhtml-invalid-missing-alt.xml`
- `docbook-valid-article.xml` / `docbook-valid-book.xml`
- `docbook-invalid-bad-gYear.xml`
- `xsd-valid-all-types.xml` / `xsd-invalid-types.xml`
