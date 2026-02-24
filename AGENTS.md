# AGENTS.md

## Project Overview

This is a Rust implementation of the [RELAX NG](https://relaxng.org/) XML schema validation language. It parses RELAX NG schemas (compact `.rnc` and XML `.rng` syntax) and validates XML documents against them.

### Crate Architecture

```
relaxng-syntax   Parsing .rnc (compact) and .rng (XML) into an AST
       |
relaxng-model    Compiles AST into a simplified pattern tree; handles
       |         includes, grammars, datatypes, error reporting
relaxng-validator  Validates XML documents against compiled schemas
       |           using derivative-based pattern matching
relaxng-tool     CLI entry point: `rng validate schema.rnc input.xml`
```

Dependencies flow downward. Each crate has a single responsibility.

### Key Source Files

| File | Lines | Role |
|---|---|---|
| `relaxng-syntax/src/types.rs` | AST types: `Schema`, `Pattern`, `Decl`, etc. |
| `relaxng-syntax/src/compact.rs` | `nom`-based parser for compact syntax |
| `relaxng-syntax/src/xml.rs` | `roxmltree`-based parser for XML syntax |
| `relaxng-model/src/lib.rs` | `Compiler` -- compiles syntax into model patterns |
| `relaxng-model/src/model.rs` | Core model: `Pattern`, `NameClass`, `DefineRule` |
| `relaxng-model/src/datatype/` | `Datatype` trait + XSD/RELAX NG builtin types |
| `relaxng-validator/src/lib.rs` | `Validator` -- streaming derivative-based validation |
| `relaxng-validator/tests/spectest.rs` | Custom test harness for the 384-case RELAX NG conformance suite |

---

## Reference Specification & Canonical Implementation

When in doubt about correct behavior, consult these authoritative sources:

### RELAX NG Specification

- **Spec home:** <https://relaxng.org/>
- **Section 7 (Restrictions):** The normative rules for what constitutes a valid simplified schema. This is the primary reference for Phase 1 work. See the [full spec](https://relaxng.org/spec-20011203.html), section 7.
- **Derivative algorithm:** <https://relaxng.org/jclark/derivative.html> -- James Clark's paper describing the derivative-based validation approach this codebase implements.

### Jing/Trang (Canonical Implementation)

- **Repository:** <https://github.com/relaxng/jing-trang>
- **Jing** is the canonical RELAX NG validator, written in Java by James Clark (the spec author). It is the reference implementation for correct behavior.
- **Trang** is a multi-format schema converter (RNC <-> RNG <-> XSD <-> DTD).

Use Jing/Trang as the ground truth when:
- You need to understand what the correct behavior should be for an ambiguous test case
- You want to compare error messages or validation outcomes
- You're unsure how a spec rule should apply in a specific edge case
- You need to understand how restriction checks interact with pattern simplification

To test a schema against Jing locally (if Java is available):
```bash
java -jar jing.jar schema.rng document.xml
```

The RELAX NG spectest suite used in this project (`relaxng-validator/tests/spectest/`) is the same conformance suite that Jing passes fully. Any test case we fail that Jing passes represents a bug in our implementation, not an ambiguity in the spec.

---

## Development Workflow

### Before You Start

1. Read `ROADMAP.md` for the current status, gap analysis, and prioritized phases.
2. Run all tests to establish your baseline (see below).
3. Identify which roadmap phase/gap your work addresses.

### Red/Green TDD

All work on this project **must** follow red-green-refactor:

1. **Red** -- Write a failing test that demonstrates the bug or specifies the new behavior. For spec compliance work, identify the specific spectest cases that should change outcome. For new functionality, write unit tests in the appropriate crate's test module.

2. **Green** -- Write the minimum code to make the failing test pass. Do not add unrelated changes.

3. **Refactor** -- Clean up the implementation while keeping all tests green. Extract functions, improve naming, remove duplication -- but only if tests still pass.

### Testing

#### Running Tests

```bash
# All unit tests (must all pass -- do not regress these)
cargo test

# RELAX NG conformance suite (custom harness, runs via spectest binary)
cargo test --test spectest -- spectest

# Single crate tests
cargo test -p relaxng-model
cargo test -p relaxng-validator
cargo test -p relaxng-syntax

# Benchmarks (relaxng-model only)
cargo bench -p relaxng-model
```

#### Test Baseline

The unit tests (15 tests across crates) **must always pass**. Do not commit code that regresses any unit test.

The spectest suite currently has known failures (see `ROADMAP.md` for exact counts). When working on a phase, you should:
- Track how many spectests pass before and after your change
- Never regress the spectest pass count without a clear, documented reason
- Report the new pass/fail/skip counts in your commit message

The spectest harness is a custom binary (`harness = false` in `relaxng-validator/Cargo.toml`). It only runs when invoked as `cargo test --test spectest -- spectest` (note the argument filtering in `spectest.rs:18`). It outputs results to stderr.

#### Adding Tests

- **Unit tests**: Add `#[test]` functions in the relevant crate. The validator crate has unit tests in `relaxng-validator/src/lib.rs` (look for `#[cfg(test)] mod tests`).
- **Integration/regression tests**: If a bug isn't covered by the spectest suite, add a focused test case that exercises the specific fix.
- **Prefer small, targeted tests** that isolate one behavior.

### Code Style

- **Edition 2024** across all crates.
- Run `cargo fmt --all` before committing. CI enforces `cargo fmt --all -- --check`.
- Run `cargo clippy` and address warnings where practical.
- Write idiomatic Rust: use `Result` for fallible operations, avoid `unwrap()` in library code (it's acceptable in tests), prefer pattern matching over `if let` chains where it improves clarity.
- Follow existing naming conventions and module structure.
- Document public APIs with doc comments. Internal functions benefit from a brief comment if the logic is non-obvious.
- Keep functions focused and short. If a function grows beyond ~50 lines, consider extracting helpers.

### Committing

Before every commit:

```bash
cargo fmt --all
cargo test          # unit tests must pass
cargo test --test spectest -- spectest  # note spectest results
```

Commit messages should:
- Summarize the change in the first line (< 72 chars)
- Reference the roadmap phase if applicable (e.g., "Phase 1: implement start-element restriction check")
- Include spectest impact if relevant (e.g., "spectest: 266 -> 278 passed")

---

## Architecture Notes

### Pattern Compilation (`relaxng-model`)

The `Compiler<FS>` is generic over a `Files` trait for filesystem access (enabling test mocking). Compilation flow:

1. Parse `.rng`/`.rnc` file into AST (`relaxng-syntax`)
2. Walk AST, resolve `include`/`externalRef`, expand grammars
3. Produce a simplified `Pattern` tree (see `relaxng-model/src/model.rs`)
4. Patterns use `PatRef = Rc<RefCell<DefineRule>>` for recursive definitions

Errors are represented by `RelaxError` enum with span information for diagnostics.

### Derivative-Based Validation (`relaxng-validator`)

The validator implements the [derivative algorithm](https://relaxng.org/jclark/derivative.html):

- Walks XML tokens from `xmlparser` (streaming, no tree construction)
- Computes pattern derivatives for each XML event (start-element, attribute, text, end-element)
- Uses memoization via `Schema::memo` HashMap for pattern deduplication
- `Pat` enum represents runtime patterns; `PatId` is used for efficient comparison

### Datatype System (`relaxng-model/src/datatype/`)

- `Datatype` trait defines the interface for type checking
- `relax.rs`: RELAX NG builtins (`string`, `token`)
- `xsd.rs`: XML Schema datatypes with facet support
- Known issue: unsupported XSD types currently panic (see Phase 4 in roadmap)

### Error Reporting

Uses `codemap` + `codemap-diagnostic` for source-span-aware error messages. Both the compiler and spectest harness produce diagnostics with file positions.

---

## Roadmap Reference

See `ROADMAP.md` for the full gap analysis and phased improvement plan. In summary:

| Phase | Focus | Impact |
|---|---|---|
| 1 | Section 7 restriction checking | ~87 test fixes |
| 2 | Namespace handling in validation | ~22 test fixes |
| 3 | Invalid input acceptance fixes | ~6 test fixes |
| 4 | Datatype robustness (eliminate panics) | ~2 test fixes |
| 5 | NCName & URI validation | ~4 test fixes |
| 6 | Performance optimization | No test fixes, real-world usability |

When starting a new phase, re-read the relevant section of `ROADMAP.md` for specific guidance on where to make changes and which test cases to target.
