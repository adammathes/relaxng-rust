# RELAX NG Validator Roadmap

## Current Status (February 2026)

The spectest suite (from the official RELAX NG test suite) shows:

| Metric   | Count |
|----------|-------|
| Passed   | 384   |
| Failed   | 0     |
| Skipped  | 0     |
| **Total**| **384** |
| **Pass rate** | **100%** |

### Progress history

| Milestone | Passed | Failed | Pass rate |
|---|---|---|---|
| Baseline (pre-Phase 1) | 266 | 118 | 69.3% |
| After Phase 1 (Section 7 restrictions) | 348 | 36 | 90.6% |
| After Phase 2 (Namespace + validation fixes) | 375 | 9 | 97.7% |
| After Phase 3 (NCName/URI validation) | 381 | 3 | 99.2% |
| After Phase 4 (QName namespace context) | 384 | 0 | 100.0% |

---

## Completed Work

### Phase 1: Section 7 Restriction Checking (DONE)

**Impact: +82 tests fixed (from 69.3% to 90.6%)**

Added `relaxng-model/src/restrictions.rs` -- a post-compilation pass that walks
the simplified pattern tree and enforces Section 7 restrictions:

- **Section 7.1.5:** Start pattern must contain element
- **Section 7.1.1:** `xmlns` attribute forbidden in all name class forms
- **Section 7.1.2:** Attribute nesting and duplicate attribute checks
- **Section 7.1.3:** List content restrictions (no nested list, no element/attribute/text/interleave)
- **Section 7.1.4:** `except` in `data` content restrictions
- **Section 7.1:** Interleave restrictions (text overlap, element overlap, attribute overlap)
- **Section 7.2/7.3:** Element/attribute overlap in groups, duplicate definitions

### Phase 2: Namespace Handling + Validation Fixes (DONE)

**Impact: +27 tests fixed (from 90.6% to 97.7%)**

Fixed namespace resolution, validation edge cases, and `externalRef` ns propagation:

- **`ns` attribute propagation:** Fixed `compile_nameclass` to apply context default
  namespace to unqualified element names during compilation
- **`externalRef` ns:** Properly passes `ns` attribute as default namespace when
  compiling externally referenced schemas
- **`xml:` prefix:** Added predefined `xml` -> `http://www.w3.org/XML/1998/namespace`
  binding in the validator
- **`data`/`value` type="string":** Fixed empty text matching for string datatype
- **Processing instructions:** Fixed text buffering in validator to include PI-adjacent text
- **`list` with `empty`:** Handled `list(empty)` as matching empty text
- **Empty element text:** Fixed validator to synthesize empty text event for elements
  with text-expecting content patterns
- **Recursive ref handling:** Fixed restriction checker to avoid infinite loops on
  self-referential `<ref>` patterns

---

## Remaining Gap Analysis

### Gap 1: NCName Validation (5 failures)

Element/attribute names and define/ref names must be valid XML NCNames. The
parser accepts names containing characters like `U+0E35` (Thai combining mark)
in positions where they are not valid NCName start characters.

### Gap 2: `datatypeLibrary` URI Validation (1 failure)

The `datatypeLibrary` attribute must be an absolute URI. Values like `"foo:"`
are accepted but should be rejected.

### Gap 3: Datatype Namespace Handling (1 failure)

One test case where invalid input is accepted due to datatype namespace context
not being properly passed during validation.

### Gap 4: Panics in Datatype Handling (2 failures)

Two test cases cause panics (thread failures) in the datatype module. Likely
hitting unimplemented datatypes or facets. The README already notes this:
unsupported XSD datatype cases currently panic rather than producing errors.

### Gap 5: Performance (not tested in spectest)

The README notes two known issues:
- Generally slow compared to Jing
- Exponential blowup for certain pathological schemas (the `blowup` unit test
  is `#[ignore]`d for this reason)

---

## Remaining Roadmap

### Phase 3: NCName & URI Validation (DONE)

**Impact: +6 test fixes (from 97.7% to 99.2%)**

1. **NCName start-character validation** -- Added full XML 1.0 Second Edition
   character table in `relaxng-syntax/src/ncname.rs`. Correctly rejects names
   starting with characters that are valid XML name chars but not NCName start
   chars (e.g., U+0E35 Thai combining mark).

2. **`datatypeLibrary` URI validation** -- Reject bare "scheme:" URIs (empty
   path, no authority) which are invalid per RFC 2396.

### Phase 4: Datatype Namespace Context (DONE)

**Impact: +3 test fixes (from 99.2% to 100%)**

1. **QName namespace-aware compilation** -- `QNameVal` now stores the expanded
   `(namespace_uri, local_name)` pair instead of the lexical string. Schema-side
   namespace context (in-scope namespace declarations + RELAX NG `ns` attribute)
   is threaded from `DatatypeValuePattern` through the compiler.

2. **QName namespace-aware validation** -- Instance-side namespace context from
   the validator's `ElementStack` is passed to `text_deriv` and used for
   namespace-aware QName comparison.

### Phase 5: Real-World Schema Testing

**Goal: validate the implementation against real schemas and real documents;
find and fix bugs not covered by the spectest suite.**

The spectest suite exercises the RELAX NG *specification* exhaustively but uses
synthetic schemas and tiny documents. Real-world schemas exercise different
failure modes: deeply recursive grammars, mixed-content prose, XSD datatype
coverage, multi-file includes, and large documents.

#### Methodology

Use **Jing** (the reference Java implementation) as a ground-truth oracle:

```
# Install: apt install jing  OR  brew install jing
jing schema.rng document.xml   # exit 0 = valid, exit 1 = invalid
cargo run -p relaxng-tool -- validate schema.rng document.xml
```

A divergence (one tool says valid, the other invalid) indicates a bug. Collect
a small test corpus of `(schema, document, expected_result)` triples and wire
them into `tests/real_world.rs` so regressions are caught automatically.

#### Target schemas (priority order)

**Tier 1 — start here (schemas are small/medium, documents easy to obtain)**

| Schema | Why it's valuable | Key edge cases |
|---|---|---|
| **Atom 1.0** (RFC 4287) | Ubiquitous feed format; concise schema | `xsd:anyURI`, `xsd:dateTime`, namespace extension points |
| **XHTML 1.0 Strict** | Real prose documents; well-known | Mixed content, `id`/`idref`, character data rules |
| **DocBook 5.x** (`.rnc`) | Recursive sections, rich attribute set | Deep recursion, complex name classes, `xsd:NMTOKEN` |

**Tier 2 — heavier stress tests**

| Schema | Why it's valuable | Key edge cases |
|---|---|---|
| **ODF / OpenDocument** | ISO standard; massive interleave | 10+ namespaces, measurement datatypes, large real `.odt` files |
| **TEI P5** | Extreme pattern complexity | Deeply nested choice/interleave, large scholarly XML |
| **DITA** (base topic) | Multi-file `include`, specialisations | Override resolution, modular grammar assembly |

#### Expected bug surfaces

Based on what real-world schemas exercise versus the spectest corpus:

1. **XSD datatype coverage** — real schemas routinely use `xsd:date`,
   `xsd:integer`, `xsd:decimal`, `xsd:anyURI`, `xsd:NMTOKEN`, `xsd:language`,
   `xsd:ID`/`xsd:IDREF`. Several of these currently panic instead of
   returning errors (README note). Fixing panics → proper errors is a
   prerequisite for real-world use.

2. **Mixed-content documents** — prose XML with text nodes interleaved between
   elements. The spectest has limited mixed-content coverage; DocBook/XHTML
   documents will exercise this path heavily.

3. **Multi-file schemas via `include`** — DocBook, DITA, and TEI all split the
   schema across many files. This exercises the `Compiler`'s file-resolution
   and include-override logic under real conditions.

4. **`anyName` / `nsName` wildcards** — ODF and DITA use open content models
   (`anyName` with `except`). The restriction checker and validator both need
   to handle these in the context of large real documents.

5. **Error message quality** — the spectest only checks pass/fail. Real-world
   use requires actionable error messages: which element, which attribute,
   what was expected. A secondary goal is comparing error *locations* against
   Jing output.

6. **Stack overflow on large/deep documents** — the derivative algorithm is
   recursive. A DocBook book or large ODF spreadsheet may overflow the default
   stack. May require `stacker` or iterative reformulation.

#### Deliverables

- `tests/real_world.rs` — integration test file, initially gated behind
  `#[ignore]` or a feature flag so CI doesn't require internet access
- A `testdata/real-world/` directory with self-contained `(schema, document)`
  pairs at small enough size to commit
- A `BUGS.md` or issue list tracking every divergence found vs. Jing

---

### Phase 6: Performance

**Impact: no test fixes, but critical for real-world use**

1. **Memoization / caching of derivatives** -- The derivative-based validation
   algorithm can benefit from caching `deriv(pattern, token)` results.

2. **Pattern interning** -- The `Pat` type is 208 bytes (per the TODO in the
   validator). Shrink it or use arena allocation.

3. **Exponential blowup mitigation** -- Implement the technique from
   [James Clark's paper](https://relaxng.org/jclark/derivative.html) to avoid
   exponential blowup. Enable the `blowup` test.

---

## Summary

| Phase | Focus | Test fixes | Cumulative pass rate |
|---|---|---|---|
| Baseline | -- | -- | 69.3% (266/384) |
| Phase 1 (done) | Section 7 restrictions | +82 | 90.6% (348/384) |
| Phase 2 (done) | Namespace + validation fixes | +27 | 97.7% (375/384) |
| Phase 3 (done) | NCName/URI validation | +6 | 99.2% (381/384) |
| Phase 4 (done) | QName namespace context | +3 | 100% (384/384) |
| Phase 5 | Real-world schema testing | bugs TBD | 100% + real-world coverage |
| Phase 6 | Performance | +0 | ~100% (+ real-world usability) |
