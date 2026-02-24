# Phase 5 Bug Report: Real-World Schema Testing

**Methodology:** Each schema/document pair was validated with both Jing (reference Java
implementation, the ground truth oracle) and our Rust tool. Divergences indicate bugs.
Test corpus created in `testdata/real-world/`.

---

## Summary Table

| # | Severity | Category | Status |
|---|----------|----------|--------|
| B1 | **Critical** | CLI: `.rng` extension not recognized → always uses compact parser | **FIXED** |
| B2 | **High** | Section 7: `Ref` content-type false positive blocks real-world schemas | open |
| B3 | **High** | Validator panics on `NMTOKEN`/`NMTOKENS` validation | open |
| B4 | **Medium** | 17 XSD datatypes fail at schema-compile time with "Unsupported" | open |
| B5 | **Medium** | `positiveInteger` incorrectly accepts 0 | open |
| B6 | **Medium** | XSD `pattern` facet matches substrings instead of full value | open |
| B7 | **Low** | `anyURI` rejects values with spaces (Jing accepts them) | open |

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

## Bug #2 — Section 7 "string sequence" false positive for `Ref` patterns

**Severity:** High — blocks compilation of any schema that uses an attribute-group ref
adjacent to a `data`/`value` element content pattern. This affects the Atom 1.0 schema
and the pattern is extremely common in real-world schemas.

**Root cause:** In `relaxng-model/src/restrictions.rs`, the `content_type()` function
unconditionally returns `ContentType::Complex` for `Pattern::Ref`:
```rust
Pattern::Ref(_, _, _) => ContentType::Complex,
```
The RELAX NG spec says Section 7 checks must be applied to the fully-simplified schema
(with all refs inlined). When a ref points to an attribute-only group (which has
`ContentType::Empty`), treating it as `Complex` makes the adjacent `data` (Simple)
appear to violate the "groupable" rule: `groupable(Complex, Simple) = false`.

**Minimal reproducer:** (Jing: valid — Tool: "string sequence" error)
```xml
<grammar xmlns="http://relaxng.org/ns/structure/1.0"
         datatypeLibrary="http://www.w3.org/2001/XMLSchema-datatypes">
  <start>
    <element name="root">
      <ref name="attrs"/>          <!-- attribute-only ref → ContentType::Empty -->
      <data type="integer"/>       <!-- Simple -->
    </element>
  </start>
  <define name="attrs">
    <optional><attribute name="id"><data type="string"/></attribute></optional>
  </define>
</grammar>
```

**Impact:** The Atom 1.0 schema (and any schema with a pattern like
`atomCommonAttributes`) fails to compile. This manifests as:
```
error: Pattern 'string sequence' is not allowed in 'group/interleave' context (section 7)
 --> atom.rng:11:4
```

**Fix direction:** `content_type()` must follow `Pattern::Ref` and compute the actual
content type of the referenced definition. Circular refs must be handled (a ref that
eventually contains an element → Complex; a ref that only contains attributes → Empty).
Since refs already track `DefineRule` via `Rc<RefCell<...>>`, this is straightforward
but requires a `seen: HashSet<usize>` to break cycles.

---

## Bug #3 — `NMTOKEN` and `NMTOKENS` panic at validation time

**Severity:** High — panics crash the process. Schemas using `NMTOKEN`/`NMTOKENS`
(XHTML class/rel/rev attributes, DocBook role attributes, many others) compile without
error but crash on the first document that exercises those attributes.

**Root cause:** In `relaxng-model/src/datatype/xsd.rs`, `is_valid()` for these two
variants calls `unimplemented!()`:
```rust
XsdDatatypes::NmTokens(_len) => { unimplemented!() }
XsdDatatypes::NmToken(_len) => { unimplemented!() }
```
Similarly, `XsdDatatypes::Token(_len)` has `unimplemented!()` (though `token` seems to
work in practice — the panic branch for `Token` may not be reached in normal validation).

**Affected XHTML documents:** Any document with `class="..."`, `rel="..."`, `rev="..."`,
`meta name="..."` attributes.

**Minimal reproducer:**
```xml
<!-- schema with NMTOKEN -->
<grammar ...>
  <start>
    <element name="r">
      <attribute name="v"><data type="NMTOKEN"/></attribute>
    </element>
  </start>
</grammar>
```
```xml
<!-- document -->
<r v="my-token"/>
<!-- Panics at xsd.rs:141: not implemented -->
```

**Fix direction:** Implement proper NMTOKEN and NMTOKENS validation:
- `NMTOKEN`: any XML name token — matches `[a-zA-Z0-9._:\-]+` (simplified; full XML
  namechar set per XML spec)
- `NMTOKENS`: one or more whitespace-separated NMTOKENs

These can be validated with existing `is_valid_ncname` infrastructure plus regex.

---

## Bug #4 — 17 XSD datatypes fail at schema compile time

**Severity:** Medium — schemas using these types fail to compile at all (unlike Bug #3
where compilation succeeds but validation panics). The error is graceful ("Unsupported
datatype") rather than a panic, but it prevents use of the schema.

**Unsupported datatypes (as of this testing):**

| Datatype | Real-world usage |
|----------|-----------------|
| `time` | Time-of-day fields |
| `Name` | XML name validation |
| `nonNegativeInteger` | Table attributes, counters |
| `negativeInteger` | Negative-only integers |
| `nonPositiveInteger` | Non-positive numbers |
| `byte` | Small signed integers |
| `unsignedByte` | Small unsigned integers (0–255) |
| `float` | Single-precision floating point |
| `base64Binary` | Embedded binary data (ODF, SOAP) |
| `hexBinary` | Hex-encoded binary |
| `gYear` | Year-only dates (copyright years) |
| `gYearMonth` | Year-month dates |
| `gMonth` | Month-only dates |
| `gMonthDay` | Month-day dates |
| `gDay` | Day-only dates |
| `QName` | Qualified XML names (see note) |
| `ENTITY` | DTD entity references |

**Note on `QName`:** Phase 4 added support for `<value type="QName">` (the
`DatatypeValue` path). The `<data type="QName">` path (`DatatypeName`) is still
unsupported.

**Most impactful gaps for real-world use:**
1. `float` — used in scientific/technical schemas
2. `nonNegativeInteger` — extremely common (CSS measurements, counters)
3. `gYear` — DocBook copyright years, metadata
4. `base64Binary` / `hexBinary` — binary content in ODF, OOXML, SOAP

**Fix direction:** These all have well-defined XSD semantics. The calendar types
(`gYear`, `gYearMonth`, etc.) can be validated with regex; `float` maps to Rust `f32`;
`nonNegativeInteger` is BigUint ≥ 0; `base64Binary`/`hexBinary` are regex matches.

---

## Bug #5 — `positiveInteger` incorrectly accepts 0

**Severity:** Medium — silent false-positive (accepts invalid documents).

**Root cause:** The `positive_integer()` builder in `xsd.rs` creates a `MinMaxFacet<BigUint>`
with no minimum constraint. `BigUint::from_str("0")` succeeds and `is_valid()` returns
`true` for 0. The implicit minimum of 1 (positiveInteger = nonNegativeInteger ≥ 1) is
never applied.

**Reproducer:** (Jing: invalid — Tool: valid)
```xml
<element name="r"><attribute name="v"><data type="positiveInteger"/></attribute></element>
```
```xml
<r v="0"/>  <!-- positiveInteger must be ≥ 1, so 0 is invalid -->
```

**Fix direction:** In `positive_integer()`, add:
```rust
min_max.min_inclusive(BigUint::from(1u32))?;
```
before processing user-supplied facets (user facets can only tighten, not loosen the
base constraint).

---

## Bug #6 — XSD `pattern` facet matches substrings, not full values

**Severity:** Medium — silent false-positive (accepts invalid documents when pattern
constraints are present). Any schema using `pattern` facets (common in healthcare, financial
standards) will silently accept malformed data.

**Root cause:** The regex compiled from the `pattern` facet parameter is applied
unanchored. XSD specifies that `pattern` must match the ENTIRE lexical value
(implicit `^...$` anchoring). The tool matches if the pattern appears *anywhere* in
the string.

**Reproducer:** (Jing: invalid — Tool: valid)
```xml
<data type="string">
  <param name="pattern">[A-Z]{2}-[0-9]{4}</param>
</data>
```
```
"AB-12345"  → should be INVALID (5 digits, pattern requires exactly 4)
"ABC-1234"  → should be INVALID (3 letters, pattern requires exactly 2)
```
Both are accepted by the tool because the substring "AB-1234" matches within "AB-12345".

**Fix direction:** When compiling a `pattern` param into a regex, wrap with `^` and `$`
anchors:
```rust
Regex::new(&format!("^(?:{pattern})$", pattern = param_value))
```
Note: XSD patterns use XSD regex syntax (a subset of standard regex), not full POSIX/PCRE.
Anchoring is the simplest fix; a more complete fix validates that only XSD regex constructs
are used.

---

## Bug #7 — `anyURI` rejects values with spaces

**Severity:** Low — Jing (following XSD 1.0 practice) is permissive about `anyURI`
and accepts any string. Our tool is stricter and rejects strings containing spaces.
Per XSD, `anyURI` values undergo whitespace collapse (`replace` → `collapse` is
`preserve` actually), and many implementations treat `anyURI` as basically any string.

**Reproducer:** (Jing: valid — Tool: invalid)
```xml
<attribute name="u"><data type="anyURI"/></attribute>
```
```
u="not a uri with spaces"   → Jing: valid, Tool: invalid
```

**Fix direction:** Either relax the `anyURI` validator to accept any Unicode string
(matching Jing's behavior), or research the XSD spec and implement the correct permissive
interpretation. The XSD 1.1 spec is more explicit that `anyURI` accepts any string.

---

## Schema / Tool Coverage Results

After fixing Bug #1, the following was confirmed to work correctly:

| Schema | Valid docs pass | Invalid docs rejected | Notes |
|--------|----------------|----------------------|-------|
| Atom 1.0 | **No** (Bug #2 blocks compile) | — | atom.rng can't compile |
| XHTML 1.0 Strict (simple) | **Yes** | **Yes** (missing alt) | |
| XHTML 1.0 Strict (table) | **PANIC** | — | Bug #3 (NMTOKEN) |
| XHTML 1.0 Strict (ids) | **PANIC** | — | Bug #3 (NMTOKENS) |
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
