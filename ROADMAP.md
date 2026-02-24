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

### Phase 5: Performance

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
| Phase 5 | Performance | +0 | ~100% (+ real-world usability) |
