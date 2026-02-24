# RELAX NG Validator Roadmap

## Current Status (February 2026)

The spectest suite (from the official RELAX NG test suite) shows:

| Metric   | Count |
|----------|-------|
| Passed   | 266   |
| Failed   | 118   |
| Skipped  | 0     |
| **Total**| **384** |
| **Pass rate** | **69.3%** |

### Breakdown by failure type

| Failure mode | Count | Description |
|---|---|---|
| Incorrect schema accepted | 87 | Schema should have been rejected but was not |
| Valid input rejected | 22 | Document matching the schema was wrongly rejected |
| Invalid input accepted | 6 | Document violating the schema was wrongly accepted |
| Test thread panicked | 2 | Crash during test execution |
| Correct schema rejected | 1 | Valid schema was wrongly rejected during compilation |

### Breakdown by spec section

| Section | Passed | Failed | Notes |
|---|---|---|---|
| Various possible syntax errors | 15 | 0 | Fully passing |
| Tests for obsolete syntax | 8 | 0 | Fully passing |
| Tests for missing attributes and child elements | 25 | 0 | Fully passing |
| Tests for single pattern child elements | 3 | 0 | Fully passing |
| Regressions | 2 | 0 | Fully passing |
| Datatype error reporting | 5 | 0 | Fully passing |
| **Foreign element and attribute handling** | **180** | **107** | **Largest gap** |
| Tests for QName and NCNames in schemas | 7 | 5 | NCName character validation |
| Checking of ns attribute | 3 | 1 | Namespace resolution during validation |
| Checking of datatypeLibrary attribute | 15 | 1 | URI validation |
| Validation error reporting | 3 | 1 | Attribute wildcard matching |
| Datatype problems | 0 | 2 | Panics in datatype handling |

---

## Gap Analysis

### Gap 1: Section 7 Restriction Checking (87 failures)

The RELAX NG spec (section 7) defines restrictions that must be enforced on
simplified schemas. Most of these are **not yet implemented**. The existing TODO
comment in `relaxng-model/src/lib.rs:26-37` already tracks some of these. They
break down as follows:

#### 1a. `xmlns` attribute restrictions (42 cases) -- Section 7.1.1

Schemas must not define attributes named `xmlns` or in the
`http://www.w3.org/2000/xmlns` namespace. The current check in
`relaxng-syntax/src/xml.rs:323-334` only catches the simple case where
`<attribute name="xmlns">` is used directly. It misses:

- `<name>` child element containing `xmlns` (e.g., `<attribute><name>xmlns</name>...</attribute>`)
- `<name>` with whitespace around `xmlns` (e.g., `<name>\n  xmlns\n</name>`)
- `xmlns` appearing in `<choice>` alternatives within an attribute name class
- `<nsName>` or `<anyName>` patterns in attribute context that would match
  `xmlns` after `<except>` exclusion still covers it
- `<oneOrMore>` or `<zeroOrMore>` wrapping `<attribute>` with `<anyName>` minus
  `<except><name>xmlns</name></except>` (the except doesn't help -- the
  attribute can still repeat and match xmlns)

**Where to fix:** Add a post-compilation pass in `relaxng-model/src/lib.rs` that
walks the pattern tree and checks attribute name classes for xmlns violations.

#### 1b. Interleave restrictions (12 cases) -- Section 7.1

The spec restricts what can appear inside `interleave` patterns:
- `text` cannot appear in both branches of an interleave
- `element` patterns in different branches must not overlap
- Various nesting rules for interleave within interleave

**Status:** Not implemented.

#### 1c. List restrictions (11 cases) -- Section 7.1.3

The spec forbids certain patterns inside `list`:
- `list` cannot contain another `list`
- `list` cannot contain `element`, `attribute`, `text`, `interleave`
- Nested `group`/`oneOrMore` etc. inside `list` must also be checked recursively

**Status:** Not implemented.

#### 1d. `except` content restrictions (4 cases) -- Section 7.1.4

The `except` child of `data` must not contain attribute, element, text, list,
group, interleave, oneOrMore, or empty patterns.

**Status:** Not implemented.

#### 1e. Start element restriction (12 cases) -- Section 7.1.5

The `start` pattern in a grammar must be "element-contentful" -- it must
ultimately reduce to an `element` pattern (not `text`, `data`, `value`,
`empty`, `oneOrMore`, etc. at the top level). This includes checking that
`<start>` doesn't contain bare `<text/>`, `<data type="string"/>`,
`<value>foo</value>`, `<empty/>`, or `<oneOrMore>` of elements (which could
match zero elements).

Examples of schemas that should be rejected but are currently accepted:
```xml
<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start><text/></start>
</grammar>

<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start><empty/></start>
</grammar>

<grammar xmlns="http://relaxng.org/ns/structure/1.0">
  <start><data type="string"/></start>
</grammar>
```

**Status:** Not implemented.

#### 1f. `anyName`/`nsName` except restrictions (2 cases) -- Section 7.1.1

- `except` inside `anyName` must not contain `anyName` descendants
- `except` inside `nsName` must not contain `nsName` or `anyName` descendants

The XML syntax parser (`relaxng-syntax/src/xml.rs:184-188`) partially checks
`anyName` in except but does not check `nsName` restrictions.

#### 1g. NCName validation (3 cases)

Element/attribute names and define/ref names must be valid XML NCNames. The
parser accepts names containing characters like `U+0E35` (Thai combining mark)
in positions where they are not valid NCName start characters.

#### 1h. `datatypeLibrary` URI validation (1 case)

The `datatypeLibrary` attribute must be an absolute URI. Values like `"foo:"`
are accepted but should be rejected.

---

### Gap 2: Namespace Handling in Validation (22 failures)

Valid documents are being rejected during validation, mostly due to namespace
resolution issues:

#### 2a. Element namespace matching (13 cases)

The validator fails to match elements when namespaces are specified using:
- The `ns` attribute on schema elements (e.g., `<element name="foo" ns="http://example.com">`)
- `<name ns="...">` child elements
- `<nsName ns="...">` patterns
- Prefixed names (e.g., `name="eg:foo"` with `xmlns:eg="..."`)

The root cause appears to be in how compiled patterns represent namespaces vs.
how the validator resolves them from the XML instance. The `contains()` function
in `relaxng-validator/src/lib.rs:474` compares namespace URIs, but the schema
compilation may not always propagate the `ns` attribute correctly into the
`NameClass::Named` structure.

#### 2b. `xml:` prefix not predefined (1 case)

The validator does not recognize the `xml:` prefix as predefined (bound to
`http://www.w3.org/XML/1998/namespace`). Documents using `xml:lang` are
rejected with `UndefinedNamespacePrefix`.

#### 2c. `list` containing `empty` (1 case)

`<element name="foo"><list><empty/></list></element>` rejects `<foo/>`. The
validator does not handle `list` containing `empty` correctly during text
matching.

#### 2d. `data type="string"` and `value type="string"` matching (3 cases)

Elements with `<data type="string"/>` or `<value type="string"/>` or `<value/>`
as their content pattern reject empty elements. The built-in `string` type
should match any text content including empty text.

#### 2e. Processing instructions in text content (2 cases)

Documents containing processing instructions (`<?target data?>`) interleaved
with text are rejected. The validator likely doesn't skip processing
instructions properly during text-node handling.

#### 2f. Attribute wildcard in validation reporting (1 case)

One test in the "Validation error reporting" section fails, involving multiple
attributes matched by wildcard patterns.

---

### Gap 3: Invalid Input Wrongly Accepted (6 failures)

Six test cases where invalid documents are accepted:

- `externalRef` with `ns` attribute not applied to referenced schema (1 case)
- Grammar-level namespace propagation issues (2 cases)
- Element namespace filtering with `nsName` patterns (2 cases)
- Datatype validation for namespaced content (1 case)

These are closely related to the namespace propagation issues in Gap 2.

---

### Gap 4: Panics in Datatype Handling (2 failures)

Two test cases cause panics (thread failures) in the datatype module. Likely
hitting unimplemented datatypes or facets. The README already notes this:
unsupported XSD datatype cases currently panic rather than producing errors.

---

### Gap 5: Performance (not tested in spectest)

The README notes two known issues:
- Generally slow compared to Jing
- Exponential blowup for certain pathological schemas (the `blowup` unit test
  is `#[ignore]`d for this reason)

---

## Roadmap

### Phase 1: Section 7 Restriction Checking

**Impact: ~87 test fixes (from 69% to ~92%)**

Add a post-compilation validation pass in `relaxng-model` that walks the
simplified pattern tree and enforces section 7 restrictions. This is the single
highest-impact workstream.

Implementation plan:

1. **Add a `check_restrictions()` function** in `relaxng-model/src/lib.rs` (or
   a new `restrictions.rs` module) that traverses the compiled `Pattern` tree.

2. **Implement checks in priority order:**

   | Check | Cases fixed | Complexity |
   |---|---|---|
   | Start must contain element (7.1.5) | ~12 | Low |
   | xmlns attribute forbidden (7.1.1) | ~42 | Medium |
   | Attribute nesting (7.1.2) | ~6 | Medium |
   | List content restrictions (7.1.3) | ~11 | Medium |
   | Except content restrictions (7.1.4) | ~4 | Low |
   | Interleave restrictions (7.1) | ~12 | High |

3. **Add new error variants** to `RelaxError` for each restriction violation.

4. **Call `check_restrictions()`** at the end of `Compiler::compile()`, after
   all simplification and define-resolution is complete.

### Phase 2: Namespace Handling Fixes

**Impact: ~22 test fixes (from ~92% to ~98%)**

Fix namespace propagation and matching between schema compilation and
validation.

1. **Predefined `xml:` prefix** -- Add `xml` ->
   `http://www.w3.org/XML/1998/namespace` to the validator's default namespace
   bindings. (1 fix, trivial)

2. **`ns` attribute propagation** -- Audit how `ns` attributes on `<element>`
   and `<name>` elements in the schema are compiled into `NameClass::Named`
   namespace URIs. Ensure `get_ns()` / `qname_att()` / `qname_el()` in
   `relaxng-syntax/src/xml.rs` correctly resolve inherited `ns` values.
   (13 fixes)

3. **`data`/`value` type="string" matching** -- Ensure the built-in `string`
   datatype matches empty text and that `<value/>` (default type token) matches
   empty element content. (3 fixes)

4. **Processing instruction handling** -- Skip processing instructions during
   validation text collection (or treat them as empty text). (2 fixes)

5. **`list` with `empty`** -- Handle `list(empty)` as matching empty text
   content. (1 fix)

6. **Attribute wildcard matching** -- Review `att_deriv` for correct handling of
   multiple attributes against wildcard (`anyName`) patterns. (1 fix)

### Phase 3: Invalid Input Acceptance Fixes

**Impact: ~6 test fixes**

1. **`externalRef` namespace propagation** -- The `ns` attribute on
   `<externalRef>` should set the default namespace for the referenced schema.

2. **`nsName` filtering in validation** -- Ensure `NsName` patterns with
   `except` correctly exclude matching elements/attributes.

3. **Datatype namespace handling** -- Review how namespace context is passed
   during datatype validation of QName values.

### Phase 4: Datatype Robustness

**Impact: ~2 test fixes + general robustness**

1. **Replace panics with errors** -- Audit all `panic!()`, `unwrap()`,
   `todo!()`, and `unimplemented!()` calls in the datatype module.  Return
   proper `RelaxError` variants instead.

2. **Implement missing XSD datatypes** -- Incrementally add support for
   commonly used XSD datatypes and facets (the spec test suite exercises at
   least `string`, `token`, `integer`, `decimal`, `QName`).

### Phase 5: NCName & URI Validation

**Impact: ~4 test fixes**

1. **NCName start-character validation** -- Reject names starting with
   characters that are valid XML name characters but not valid NCName start
   characters (e.g., Thai combining marks).

2. **`datatypeLibrary` URI validation** -- Validate that the attribute value is
   an absolute URI per RFC 3986.

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

| Phase | Focus | Est. test fixes | Cumulative pass rate |
|---|---|---|---|
| Current | -- | -- | 69% (266/384) |
| Phase 1 | Section 7 restrictions | +87 | ~92% (353/384) |
| Phase 2 | Namespace handling | +22 | ~98% (375/384) |
| Phase 3 | Invalid input fixes | +6 | ~99% (381/384) |
| Phase 4 | Datatype robustness | +2 | ~99.7% (383/384) |
| Phase 5 | NCName/URI validation | +4 | ~100% |
| Phase 6 | Performance | +0 | ~100% (+ real-world usability) |
