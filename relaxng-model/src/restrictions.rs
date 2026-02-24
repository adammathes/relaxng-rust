//! Section 7 restriction checking for RELAX NG simplified schemas.
//!
//! After compilation, the pattern tree must satisfy a set of restrictions
//! defined in section 7 of the RELAX NG specification. This module implements
//! those checks as a post-compilation pass.
//!
//! Reference: <https://relaxng.org/spec-20011203.html#section7>

use crate::RelaxError;
use crate::model::{DefineRule, NameClass, Pattern};
use std::collections::HashSet;

/// Check all section 7 restrictions on the compiled pattern tree.
///
/// Called after compilation is complete and all references are resolved.
/// The `start_span` is the source span of the start rule, used for error reporting.
pub fn check_restrictions(
    start_rule: &DefineRule,
    start_span: codemap::Span,
) -> Result<(), RelaxError> {
    let pattern = start_rule.pattern();

    // 7.1.5: start element restrictions
    let mut seen = HashSet::new();
    check_start(pattern, start_span, &mut seen)?;

    // Walk the full pattern tree for remaining restrictions (7.1.1-7.1.4, 7.3)
    let mut seen = HashSet::new();
    let ctx = WalkContext::default();
    check_pattern(pattern, &ctx, start_span, &mut seen)?;

    Ok(())
}

// --- 7.1.5: Start element restrictions ---
//
// The start pattern must only contain element, choice, ref, and notAllowed.
// Forbidden: attribute, data, value, text, list, group, interleave, oneOrMore, empty.
// We also treat Mixed (= interleave + text), Optional (= choice + empty),
// and ZeroOrMore (= choice + oneOrMore + empty) as forbidden since their
// simplified forms contain forbidden patterns.

fn check_start(
    pattern: &Pattern,
    span: codemap::Span,
    seen: &mut HashSet<usize>,
) -> Result<(), RelaxError> {
    match pattern {
        // Element is the desired content for start -- stop recursing
        Pattern::Element(_, _) => Ok(()),

        // Choice is allowed -- recurse into alternatives
        Pattern::Choice(alternatives) => {
            for alt in alternatives {
                check_start(alt, span, seen)?;
            }
            Ok(())
        }

        // NotAllowed is permitted
        Pattern::NotAllowed => Ok(()),

        // Ref: follow the reference and check the resolved pattern
        Pattern::Ref(_ref_span, _name, pat_ref) => {
            let ptr = pat_ref.0.as_ptr() as usize;
            if seen.contains(&ptr) {
                return Ok(());
            }
            seen.insert(ptr);
            if let Some(rule) = pat_ref.0.borrow().as_ref() {
                check_start(rule.pattern(), span, seen)
            } else {
                Ok(())
            }
        }

        // Everything else is forbidden under start
        Pattern::Text => Err(restricted(span, "text", "start")),
        Pattern::Empty => Err(restricted(span, "empty", "start")),
        Pattern::Attribute(_, _) => Err(restricted(span, "attribute", "start")),
        Pattern::List(_) => Err(restricted(span, "list", "start")),
        Pattern::Group(_) => Err(restricted(span, "group", "start")),
        Pattern::Interleave(_) => Err(restricted(span, "interleave", "start")),
        Pattern::OneOrMore(_) => Err(restricted(span, "oneOrMore", "start")),
        Pattern::ZeroOrMore(_) => Err(restricted(span, "zeroOrMore", "start")),
        Pattern::Optional(_) => Err(restricted(span, "optional", "start")),
        Pattern::Mixed(_) => Err(restricted(span, "mixed", "start")),
        Pattern::DatatypeValue { .. } => Err(restricted(span, "value", "start")),
        Pattern::DatatypeName { .. } => Err(restricted(span, "data", "start")),
    }
}

// --- Context-aware restriction checking for the full pattern tree ---

/// Tracking which restriction-relevant contexts we are inside.
#[derive(Default, Clone)]
struct WalkContext {
    /// Inside a `list` pattern (7.1.3)
    in_list: bool,
    /// Inside `data/except` (7.1.4)
    in_data_except: bool,
    /// Inside an `attribute` pattern (7.1.1)
    in_attribute: bool,
    /// Inside a `oneOrMore` (or `zeroOrMore`) pattern (7.1.2)
    in_one_or_more: bool,
    /// Inside a `group` or `interleave` that is itself inside a `oneOrMore` (7.1.2)
    in_one_or_more_group: bool,
}

fn check_pattern(
    pattern: &Pattern,
    ctx: &WalkContext,
    span: codemap::Span,
    seen: &mut HashSet<usize>,
) -> Result<(), RelaxError> {
    match pattern {
        Pattern::Element(name_class, content) => {
            // Check name class restrictions
            check_name_class(name_class)?;
            // Element creates a new context boundary -- reset all flags
            check_pattern(content, &WalkContext::default(), span, seen)
        }

        Pattern::Attribute(name_class, content) => {
            // 7.1.1: attribute inside attribute is forbidden
            if ctx.in_attribute {
                return Err(restricted(span, "attribute", "attribute"));
            }
            // 7.1.3: attribute inside list is forbidden
            if ctx.in_list {
                return Err(restricted(span, "attribute", "list"));
            }
            // 7.1.4: attribute inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "attribute", "data/except"));
            }
            // 7.1.2: attribute inside group/interleave inside oneOrMore is forbidden
            if ctx.in_one_or_more_group {
                return Err(restricted(span, "attribute", "oneOrMore//group"));
            }

            // 7.1.1: xmlns attribute restrictions
            check_attribute_name_class(name_class)?;

            // Check name class restrictions
            check_name_class(name_class)?;

            let mut child_ctx = ctx.clone();
            child_ctx.in_attribute = true;
            check_pattern(content, &child_ctx, span, seen)
        }

        Pattern::List(content) => {
            // 7.1.3: list inside list is forbidden
            if ctx.in_list {
                return Err(restricted(span, "list", "list"));
            }
            // 7.1.4: list inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "list", "data/except"));
            }
            let mut child_ctx = ctx.clone();
            child_ctx.in_list = true;
            check_pattern(content, &child_ctx, span, seen)
        }

        Pattern::DatatypeName { except, .. } => {
            if let Some(except_pat) = except {
                let mut child_ctx = ctx.clone();
                child_ctx.in_data_except = true;
                check_pattern(except_pat, &child_ctx, span, seen)?;
            }
            Ok(())
        }

        Pattern::Choice(alternatives) => {
            for alt in alternatives {
                check_pattern(alt, ctx, span, seen)?;
            }
            Ok(())
        }

        Pattern::Group(members) => {
            // 7.1.4: group inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "group", "data/except"));
            }
            // 7.1.2: entering group while inside oneOrMore activates the
            // oneOrMore//group//attribute restriction
            let mut child_ctx = ctx.clone();
            if ctx.in_one_or_more {
                child_ctx.in_one_or_more_group = true;
            }
            for m in members {
                check_pattern(m, &child_ctx, span, seen)?;
            }
            Ok(())
        }

        Pattern::Interleave(members) => {
            // 7.1.3: interleave inside list is forbidden
            if ctx.in_list {
                return Err(restricted(span, "interleave", "list"));
            }
            // 7.1.4: interleave inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "interleave", "data/except"));
            }
            // 7.1.2: entering interleave while inside oneOrMore activates the
            // oneOrMore//interleave//attribute restriction
            let mut child_ctx = ctx.clone();
            if ctx.in_one_or_more {
                child_ctx.in_one_or_more_group = true;
            }
            for m in members {
                check_pattern(m, &child_ctx, span, seen)?;
            }
            Ok(())
        }

        Pattern::Mixed(content) => {
            // Mixed = interleave(text, content)
            // 7.1.3: interleave (and text) inside list is forbidden
            if ctx.in_list {
                return Err(restricted(span, "interleave", "list"));
            }
            // 7.1.4: interleave inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "interleave", "data/except"));
            }
            let mut child_ctx = ctx.clone();
            if ctx.in_one_or_more {
                child_ctx.in_one_or_more_group = true;
            }
            check_pattern(content, &child_ctx, span, seen)
        }

        Pattern::OneOrMore(content) => {
            // 7.1.4: oneOrMore inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "oneOrMore", "data/except"));
            }
            let mut child_ctx = ctx.clone();
            child_ctx.in_one_or_more = true;
            check_pattern(content, &child_ctx, span, seen)
        }

        Pattern::ZeroOrMore(content) => {
            // ZeroOrMore simplifies to choice(oneOrMore(content), empty)
            // 7.1.4: oneOrMore inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "oneOrMore", "data/except"));
            }
            let mut child_ctx = ctx.clone();
            child_ctx.in_one_or_more = true;
            check_pattern(content, &child_ctx, span, seen)
        }

        Pattern::Optional(content) => check_pattern(content, ctx, span, seen),

        Pattern::Ref(_ref_span, _name, pat_ref) => {
            // Follow refs and check the resolved pattern in the current context.
            // In the fully simplified schema, refs would be forbidden in certain
            // contexts (list, data/except, attribute). But in our representation,
            // we follow refs through instead since we haven't fully simplified.
            let ptr = pat_ref.0.as_ptr() as usize;
            if seen.contains(&ptr) {
                return Ok(());
            }
            seen.insert(ptr);
            if let Some(rule) = pat_ref.0.borrow().as_ref() {
                check_pattern(rule.pattern(), ctx, span, seen)
            } else {
                Ok(())
            }
        }

        // Leaf patterns -- check context restrictions
        Pattern::Text => {
            // 7.1.3: text forbidden in list
            if ctx.in_list {
                return Err(restricted(span, "text", "list"));
            }
            // 7.1.4: text forbidden in data/except
            if ctx.in_data_except {
                return Err(restricted(span, "text", "data/except"));
            }
            Ok(())
        }

        Pattern::Empty => {
            // 7.1.4: empty forbidden in data/except
            if ctx.in_data_except {
                return Err(restricted(span, "empty", "data/except"));
            }
            Ok(())
        }

        Pattern::NotAllowed => Ok(()),

        Pattern::DatatypeValue { .. } => Ok(()),
    }
}

// --- Name class restriction checking ---

/// Check name class restrictions (7.1.1): anyName/nsName except rules
fn check_name_class(name_class: &NameClass) -> Result<(), RelaxError> {
    match name_class {
        NameClass::AnyName { except } => {
            if let Some(except) = except {
                check_anyname_except(except)?;
            }
            Ok(())
        }
        NameClass::NsName { except, .. } => {
            if let Some(except) = except {
                check_nsname_except(except)?;
            }
            Ok(())
        }
        NameClass::Alt { a, b } => {
            check_name_class(a)?;
            check_name_class(b)?;
            Ok(())
        }
        NameClass::Named { .. } => Ok(()),
    }
}

// --- 7.1.1: xmlns attribute restrictions ---
//
// An attribute must not be named "xmlns" (with empty namespace) or be in
// the "http://www.w3.org/2000/xmlns" namespace.

const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns";

fn check_attribute_name_class(name_class: &NameClass) -> Result<(), RelaxError> {
    match name_class {
        NameClass::Named {
            namespace_uri,
            name,
        } => {
            if namespace_uri.is_empty() && name == "xmlns" {
                return Err(RelaxError::XmlnsAttributeForbidden);
            }
            if namespace_uri == XMLNS_NAMESPACE {
                return Err(RelaxError::XmlnsNamespaceForbidden);
            }
            Ok(())
        }
        NameClass::NsName {
            namespace_uri,
            except,
        } => {
            if namespace_uri == XMLNS_NAMESPACE {
                return Err(RelaxError::XmlnsNamespaceForbidden);
            }
            if let Some(except) = except {
                check_attribute_name_class(except)?;
            }
            Ok(())
        }
        NameClass::AnyName { except } => {
            if let Some(except) = except {
                check_attribute_name_class(except)?;
            }
            Ok(())
        }
        NameClass::Alt { a, b } => {
            check_attribute_name_class(a)?;
            check_attribute_name_class(b)?;
            Ok(())
        }
    }
}

// --- 7.1.1: anyName/nsName except restrictions ---
//
// An except inside anyName must not contain anyName descendants.
// An except inside nsName must not contain nsName or anyName descendants.

fn check_anyname_except(except: &NameClass) -> Result<(), RelaxError> {
    match except {
        NameClass::AnyName { .. } => Err(RelaxError::AnyNameInExcept),
        NameClass::Alt { a, b } => {
            check_anyname_except(a)?;
            check_anyname_except(b)?;
            Ok(())
        }
        NameClass::NsName { except, .. } => {
            if let Some(e) = except {
                check_anyname_except(e)?;
            }
            Ok(())
        }
        NameClass::Named { .. } => Ok(()),
    }
}

fn check_nsname_except(except: &NameClass) -> Result<(), RelaxError> {
    match except {
        NameClass::AnyName { .. } => Err(RelaxError::AnyNameInNsNameExcept),
        NameClass::NsName { .. } => Err(RelaxError::NsNameInNsNameExcept),
        NameClass::Alt { a, b } => {
            check_nsname_except(a)?;
            check_nsname_except(b)?;
            Ok(())
        }
        NameClass::Named { .. } => Ok(()),
    }
}

// --- Helper to construct a restriction error ---

fn restricted(span: codemap::Span, pattern_name: &str, context: &str) -> RelaxError {
    RelaxError::RestrictedPattern {
        span,
        pattern_name: pattern_name.to_string(),
        context: context.to_string(),
    }
}
