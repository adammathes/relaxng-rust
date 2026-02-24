//! Section 7 restriction checking for RELAX NG simplified schemas.
//!
//! After compilation, the pattern tree must satisfy a set of restrictions
//! defined in section 7 of the RELAX NG specification. This module implements
//! those checks as a post-compilation pass.
//!
//! Important: The section 7 restrictions apply to the _fully simplified_ schema
//! (after section 4 simplification). Since this codebase uses a partially
//! simplified representation, we must account for simplification rules when
//! checking restrictions. In particular:
//!   - group/interleave containing notAllowed → notAllowed (skip checking)
//!   - group/interleave with only empty + one real member → member (no group)
//!   - choice containing notAllowed alternatives → drop those alternatives
//!   - oneOrMore(notAllowed) → notAllowed
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

// --- Simplification-aware helpers ---

/// Returns true if a pattern simplifies to notAllowed per section 4 rules.
/// A dead pattern should not trigger restriction violations because it would
/// be eliminated during full simplification.
fn is_dead(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::NotAllowed => true,
        Pattern::Group(members) | Pattern::Interleave(members) => {
            // group/interleave with any dead member is dead
            members.iter().any(|m| is_dead(m))
        }
        Pattern::OneOrMore(p) | Pattern::List(p) => is_dead(p),
        Pattern::ZeroOrMore(_) => false, // zeroOrMore(notAllowed) = choice(oneOrMore(notAllowed), empty) = empty
        Pattern::Choice(alts) => {
            // choice is dead only if ALL alternatives are dead
            alts.iter().all(|a| is_dead(a))
        }
        Pattern::Optional(_) => false, // optional(X) = choice(X, empty), and empty is not dead
        // An attribute whose content is dead can never be satisfied
        Pattern::Attribute(_, content) => is_dead(content),
        // Element(nc, notAllowed) does NOT simplify to notAllowed per the spec.
        // The element is structurally present even if its content is notAllowed,
        // and section 7 restrictions still apply to surrounding patterns.
        Pattern::Element(_, _) => false,
        _ => false,
    }
}

/// Returns the count of "real" members in a group/interleave after removing
/// empty members (since group(X, empty) simplifies to X in section 4).
fn count_non_empty_members(members: &[Pattern]) -> usize {
    members
        .iter()
        .filter(|m| !matches!(m, Pattern::Empty))
        .count()
}

// --- 7.1.5: Start element restrictions ---
//
// The start pattern must only contain element, choice, ref, and notAllowed.
// Forbidden: attribute, data, value, text, list, group, interleave, oneOrMore, empty.
//
// We handle unsimplified patterns:
// - Optional(p) = choice(p, empty) → check p, the empty is fine in choice context
// - Group/Interleave containing notAllowed → dead, skip
// - Choice with notAllowed alternatives → skip those alternatives

fn check_start(
    pattern: &Pattern,
    span: codemap::Span,
    seen: &mut HashSet<usize>,
) -> Result<(), RelaxError> {
    // Dead patterns are fine -- they simplify to notAllowed which is allowed in start
    if is_dead(pattern) {
        return Ok(());
    }

    match pattern {
        // Element is the desired content for start -- stop recursing
        Pattern::Element(_, _) => Ok(()),

        // Choice is allowed -- recurse into non-dead alternatives
        Pattern::Choice(alternatives) => {
            for alt in alternatives {
                if !is_dead(alt) {
                    check_start(alt, span, seen)?;
                }
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

        // Optional(p) = choice(p, empty) -- check p under start rules
        Pattern::Optional(content) => check_start(content, span, seen),

        // Group/Interleave with a single non-empty member simplifies to that
        // member (section 4: group(p, empty) = p, interleave(p, empty) = p).
        // This arises from combine="choice"/"interleave" with a single definition.
        Pattern::Group(members) | Pattern::Interleave(members) => {
            let non_empty: Vec<_> = members
                .iter()
                .filter(|m| !matches!(m, Pattern::Empty))
                .collect();
            if non_empty.len() == 1 {
                return check_start(non_empty[0], span, seen);
            }
            // Multi-member group/interleave is forbidden under start
            if matches!(pattern, Pattern::Group(_)) {
                Err(restricted(span, "group", "start"))
            } else {
                Err(restricted(span, "interleave", "start"))
            }
        }

        // Everything else is forbidden under start
        Pattern::Text => Err(restricted(span, "text", "start")),
        Pattern::Empty => Err(restricted(span, "empty", "start")),
        Pattern::Attribute(_, _) => Err(restricted(span, "attribute", "start")),
        Pattern::List(_) => Err(restricted(span, "list", "start")),
        Pattern::OneOrMore(_) => Err(restricted(span, "oneOrMore", "start")),
        Pattern::ZeroOrMore(_) => Err(restricted(span, "zeroOrMore", "start")),
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
    // Skip restriction checks on dead patterns -- they would be eliminated
    // during full simplification (section 4)
    if is_dead(pattern) {
        return Ok(());
    }

    match pattern {
        Pattern::Element(name_class, content) => {
            // 7.1.3: element inside list is forbidden (list can only contain data patterns)
            if ctx.in_list {
                return Err(restricted(span, "element", "list"));
            }
            // 7.1.4: element inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "element", "data/except"));
            }
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

            // Check name class restrictions (anyName/nsName except rules)
            check_name_class(name_class)?;

            // 7.3: attribute with infinite name class (anyName/nsName) must be
            // inside oneOrMore
            if name_class_is_infinite(name_class) && !ctx.in_one_or_more {
                return Err(restricted(
                    span,
                    "attribute with infinite name class",
                    "outside oneOrMore",
                ));
            }

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
                // Skip dead alternatives -- they simplify away
                if !is_dead(alt) {
                    check_pattern(alt, ctx, span, seen)?;
                }
            }
            Ok(())
        }

        Pattern::Group(members) => {
            // 7.1.4: group inside data/except is forbidden
            if ctx.in_data_except {
                return Err(restricted(span, "group", "data/except"));
            }
            // 7.2: check string sequence restriction (content types must be groupable)
            // Inside list, group of data/value is allowed (whitespace-separated tokens)
            if !ctx.in_list {
                check_string_sequence(members, span)?;
            }
            // 7.3: check for overlapping attribute name classes within the group
            check_group_attribute_overlap(members, span)?;

            // 7.1.2: entering group while inside oneOrMore activates the
            // oneOrMore//group//attribute restriction -- BUT only if the group
            // has more than one non-empty member (otherwise it simplifies away)
            let mut child_ctx = ctx.clone();
            if ctx.in_one_or_more && count_non_empty_members(members) > 1 {
                child_ctx.in_one_or_more_group = true;
            }
            for m in members {
                if !is_dead(m) {
                    check_pattern(m, &child_ctx, span, seen)?;
                }
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
            // 7.2: check string sequence restriction (content types must be groupable)
            check_string_sequence(members, span)?;
            // 7.4: check for overlapping elements and duplicate text across
            // interleave branches
            check_interleave_restrictions(members, span)?;
            // 7.3: check for overlapping attribute name classes in interleave
            check_group_attribute_overlap(members, span)?;

            // 7.1.2: entering interleave while inside oneOrMore activates the
            // oneOrMore//interleave//attribute restriction -- only if the
            // interleave has more than one non-empty member
            let mut child_ctx = ctx.clone();
            if ctx.in_one_or_more && count_non_empty_members(members) > 1 {
                child_ctx.in_one_or_more_group = true;
            }
            for m in members {
                if !is_dead(m) {
                    check_pattern(m, &child_ctx, span, seen)?;
                }
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
            // 7.4: mixed = interleave(text, content). If content also has text
            // (e.g., content is mixed(...)), that's text in both interleave branches.
            if has_text(content) {
                return Err(restricted(
                    span,
                    "text",
                    "interleave (both branches via nested mixed)",
                ));
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

// --- 7.3: Overlapping attribute name classes in group ---
//
// In a group (or implicit group within an element's content), attribute name
// classes must not overlap between siblings. This prevents duplicate attributes.

fn check_group_attribute_overlap(
    members: &[Pattern],
    span: codemap::Span,
) -> Result<(), RelaxError> {
    let mut all_attr_names: Vec<CollectedNameClass> = Vec::new();
    for member in members {
        if is_dead(member) {
            continue;
        }
        let mut member_attrs = Vec::new();
        collect_attribute_name_classes(member, &mut member_attrs);
        for new_nc in &member_attrs {
            for existing_nc in &all_attr_names {
                if name_classes_overlap(existing_nc, new_nc) {
                    return Err(RelaxError::OverlappingAttributes { span });
                }
            }
        }
        all_attr_names.extend(member_attrs);
    }
    Ok(())
}

// --- 7.4: Interleave restrictions ---
//
// For interleave(p1, p2):
// - Element name classes from p1 and p2 must not overlap
// - Text must not appear in both p1 and p2

fn check_interleave_restrictions(
    members: &[Pattern],
    span: codemap::Span,
) -> Result<(), RelaxError> {
    // Collect element name classes and text presence from each branch
    let mut all_elem_names: Vec<CollectedNameClass> = Vec::new();
    let mut text_seen = false;

    for member in members {
        if is_dead(member) {
            continue;
        }
        // Check text overlap
        if has_text(member) {
            if text_seen {
                return Err(restricted(span, "text", "interleave (both branches)"));
            }
            text_seen = true;
        }
        // Check element name class overlap
        let mut member_elems = Vec::new();
        collect_element_name_classes(member, &mut member_elems);
        for new_nc in &member_elems {
            for existing_nc in &all_elem_names {
                if name_classes_overlap(existing_nc, new_nc) {
                    return Err(RelaxError::OverlappingElements { span });
                }
            }
        }
        all_elem_names.extend(member_elems);
    }
    Ok(())
}

// --- Name class collection and overlap detection ---

/// A simplified representation of a name class for overlap checking.
#[derive(Clone, Debug)]
enum CollectedNameClass {
    /// A specific name
    Named { namespace_uri: String, name: String },
    /// All names in a namespace, optionally with exceptions
    NsName {
        namespace_uri: String,
        except: Vec<CollectedNameClass>,
    },
    /// All names, optionally with exceptions
    AnyName { except: Vec<CollectedNameClass> },
}

/// Collect attribute name classes from a pattern tree (non-recursive into
/// element/attribute content -- only top-level attributes).
fn collect_attribute_name_classes(pattern: &Pattern, out: &mut Vec<CollectedNameClass>) {
    if is_dead(pattern) {
        return;
    }
    match pattern {
        Pattern::Attribute(nc, _) => {
            collect_name_class_entries(nc, out);
        }
        Pattern::Group(members) | Pattern::Interleave(members) | Pattern::Choice(members) => {
            for m in members {
                collect_attribute_name_classes(m, out);
            }
        }
        Pattern::OneOrMore(p)
        | Pattern::ZeroOrMore(p)
        | Pattern::Optional(p)
        | Pattern::Mixed(p) => {
            collect_attribute_name_classes(p, out);
        }
        Pattern::Ref(_, _, pat_ref) => {
            if let Some(rule) = pat_ref.0.borrow().as_ref() {
                collect_attribute_name_classes(rule.pattern(), out);
            }
        }
        // Element creates a boundary -- don't look inside
        Pattern::Element(_, _) => {}
        // Leaf patterns have no attributes
        _ => {}
    }
}

/// Collect element name classes from a pattern tree (within a single
/// interleave branch -- recurse through group/interleave/choice but
/// don't enter element content).
fn collect_element_name_classes(pattern: &Pattern, out: &mut Vec<CollectedNameClass>) {
    match pattern {
        Pattern::Element(nc, _) => {
            // Always collect element name classes, even if content is notAllowed.
            // Section 7.4 checks structural name class overlap, not semantic reachability.
            collect_name_class_entries(nc, out);
        }
        Pattern::NotAllowed => {}
        Pattern::Group(members) | Pattern::Interleave(members) => {
            // group/interleave containing NotAllowed simplifies to NotAllowed
            if members.iter().any(|m| matches!(m, Pattern::NotAllowed)) {
                return;
            }
            for m in members {
                collect_element_name_classes(m, out);
            }
        }
        Pattern::Choice(members) => {
            for m in members {
                collect_element_name_classes(m, out);
            }
        }
        Pattern::OneOrMore(p)
        | Pattern::ZeroOrMore(p)
        | Pattern::Optional(p)
        | Pattern::Mixed(p) => {
            collect_element_name_classes(p, out);
        }
        Pattern::Ref(_, _, pat_ref) => {
            if let Some(rule) = pat_ref.0.borrow().as_ref() {
                collect_element_name_classes(rule.pattern(), out);
            }
        }
        _ => {}
    }
}

/// Check if a pattern contains `text` (directly or transitively, but not
/// inside element content).
fn has_text(pattern: &Pattern) -> bool {
    if is_dead(pattern) {
        return false;
    }
    match pattern {
        Pattern::Text => true,
        Pattern::Mixed(_) => true, // mixed = interleave(text, ...)
        Pattern::Group(members) | Pattern::Interleave(members) | Pattern::Choice(members) => {
            members.iter().any(|m| has_text(m))
        }
        Pattern::OneOrMore(p) | Pattern::ZeroOrMore(p) | Pattern::Optional(p) => has_text(p),
        Pattern::Ref(_, _, pat_ref) => {
            if let Some(rule) = pat_ref.0.borrow().as_ref() {
                has_text(rule.pattern())
            } else {
                false
            }
        }
        // Element creates a boundary -- text inside element doesn't count
        Pattern::Element(_, _) => false,
        _ => false,
    }
}

/// Expand a NameClass into one or more CollectedNameClass entries.
/// Alt is expanded into separate entries so each can be checked independently.
fn collect_name_class_entries(nc: &NameClass, out: &mut Vec<CollectedNameClass>) {
    match nc {
        NameClass::Alt { a, b } => {
            collect_name_class_entries(a, out);
            collect_name_class_entries(b, out);
        }
        _ => out.push(convert_name_class(nc)),
    }
}

fn convert_name_class(nc: &NameClass) -> CollectedNameClass {
    match nc {
        NameClass::Named {
            namespace_uri,
            name,
        } => CollectedNameClass::Named {
            namespace_uri: namespace_uri.clone(),
            name: name.clone(),
        },
        NameClass::NsName {
            namespace_uri,
            except,
        } => CollectedNameClass::NsName {
            namespace_uri: namespace_uri.clone(),
            // Expand Alt in except clause to preserve all alternatives
            except: except
                .iter()
                .flat_map(|e| {
                    let mut entries = Vec::new();
                    collect_name_class_entries(e, &mut entries);
                    entries
                })
                .collect(),
        },
        NameClass::AnyName { except } => CollectedNameClass::AnyName {
            // Expand Alt in except clause to preserve all alternatives
            except: except
                .iter()
                .flat_map(|e| {
                    let mut entries = Vec::new();
                    collect_name_class_entries(e, &mut entries);
                    entries
                })
                .collect(),
        },
        NameClass::Alt { .. } => {
            // Alt at top level should be handled by collect_name_class_entries.
            // If we get here (e.g., inside an except clause), pick the most
            // restrictive alternative for conservative overlap checking.
            let mut entries = Vec::new();
            collect_name_class_entries(nc, &mut entries);
            // Return the most general one for conservative checking
            entries
                .into_iter()
                .max_by_key(|e| match e {
                    CollectedNameClass::AnyName { .. } => 2,
                    CollectedNameClass::NsName { .. } => 1,
                    CollectedNameClass::Named { .. } => 0,
                })
                .unwrap_or(CollectedNameClass::Named {
                    namespace_uri: String::new(),
                    name: String::new(),
                })
        }
    }
}

/// Check if two collected name classes could match the same name.
fn name_classes_overlap(a: &CollectedNameClass, b: &CollectedNameClass) -> bool {
    match (a, b) {
        (
            CollectedNameClass::Named {
                namespace_uri: ns1,
                name: n1,
            },
            CollectedNameClass::Named {
                namespace_uri: ns2,
                name: n2,
            },
        ) => ns1 == ns2 && n1 == n2,

        (
            CollectedNameClass::Named {
                namespace_uri,
                name,
            },
            CollectedNameClass::NsName {
                namespace_uri: ns,
                except,
            },
        )
        | (
            CollectedNameClass::NsName {
                namespace_uri: ns,
                except,
            },
            CollectedNameClass::Named {
                namespace_uri,
                name,
            },
        ) => {
            if namespace_uri != ns {
                return false;
            }
            // Named is in the namespace -- check it's not excluded
            !except.iter().any(|e| match e {
                CollectedNameClass::Named {
                    namespace_uri: ens,
                    name: en,
                } => ens == namespace_uri && en == name,
                _ => false,
            })
        }

        (
            CollectedNameClass::Named {
                namespace_uri,
                name,
            },
            CollectedNameClass::AnyName { except },
        )
        | (
            CollectedNameClass::AnyName { except },
            CollectedNameClass::Named {
                namespace_uri,
                name,
            },
        ) => {
            // AnyName matches everything unless excluded
            !is_name_excluded_by(namespace_uri, name, except)
        }

        (
            CollectedNameClass::NsName {
                namespace_uri: ns1, ..
            },
            CollectedNameClass::NsName {
                namespace_uri: ns2, ..
            },
        ) => {
            // Two nsName patterns in the same namespace always overlap
            // (unless their excepts cover each other, which is complex to check)
            ns1 == ns2
        }

        (CollectedNameClass::AnyName { .. }, CollectedNameClass::NsName { .. })
        | (CollectedNameClass::NsName { .. }, CollectedNameClass::AnyName { .. }) => {
            // anyName always overlaps with nsName (there's always at least one
            // name in the namespace that anyName matches)
            true
        }

        (CollectedNameClass::AnyName { .. }, CollectedNameClass::AnyName { .. }) => {
            // Two anyName patterns always overlap
            true
        }
    }
}

fn is_name_excluded_by(namespace_uri: &str, name: &str, excludes: &[CollectedNameClass]) -> bool {
    for exc in excludes {
        match exc {
            CollectedNameClass::Named {
                namespace_uri: ens,
                name: en,
            } => {
                if ens == namespace_uri && en == name {
                    return true;
                }
            }
            CollectedNameClass::NsName {
                namespace_uri: ns,
                except,
            } => {
                if ns == namespace_uri {
                    // The nsName excludes this namespace, unless the name is
                    // in the nsName's except list
                    let re_included = except.iter().any(|e| match e {
                        CollectedNameClass::Named {
                            namespace_uri: rns,
                            name: rn,
                        } => rns == namespace_uri && rn == name,
                        _ => false,
                    });
                    if !re_included {
                        return true;
                    }
                }
            }
            CollectedNameClass::AnyName { except } => {
                // AnyName excludes everything except what's in its except
                let re_included = is_name_excluded_by(namespace_uri, name, except);
                if !re_included {
                    return true;
                }
            }
        }
    }
    false
}

// --- 7.3: Infinite name class helpers ---

/// Check if a name class can match infinitely many names (anyName or nsName).
fn name_class_is_infinite(nc: &NameClass) -> bool {
    match nc {
        NameClass::AnyName { .. } => true,
        NameClass::NsName { .. } => true,
        NameClass::Alt { a, b } => name_class_is_infinite(a) || name_class_is_infinite(b),
        NameClass::Named { .. } => false,
    }
}

/// Check if a pattern is effectively empty (matches only the empty string
/// or nothing at all).
fn content_is_empty(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Empty | Pattern::NotAllowed => true,
        _ => false,
    }
}

// --- 7.2: Content type / string sequence checking ---
//
// Each pattern has a "content type": empty, complex, or simple.
// - empty: Empty, NotAllowed
// - complex: Element
// - simple: Data, Value, List, Text
// Group and Interleave require that their children's content types are "groupable":
//   groupable(empty, _) = true
//   groupable(_, empty) = true
//   groupable(complex, complex) = true
//   everything else = false
// So: simple is NOT groupable with simple or complex.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ContentType {
    Empty,
    Complex,
    Simple,
}

/// Compute the content type of a pattern (for section 7.2 checking).
/// Per spec: text=complex, element=complex, data/value/list=simple,
/// empty/attribute=empty, notAllowed=empty.
/// Refs are followed to their resolved content type (with cycle detection).
fn content_type(pattern: &Pattern) -> ContentType {
    let mut seen = HashSet::new();
    content_type_impl(pattern, &mut seen)
}

fn content_type_impl(pattern: &Pattern, seen: &mut HashSet<usize>) -> ContentType {
    if is_dead(pattern) {
        return ContentType::Empty;
    }
    match pattern {
        Pattern::Empty | Pattern::NotAllowed => ContentType::Empty,
        Pattern::Element(_, _) => ContentType::Complex,
        Pattern::Text => ContentType::Complex,
        Pattern::Ref(_, _, pat_ref) => {
            let ptr = pat_ref.0.as_ptr() as usize;
            if seen.contains(&ptr) {
                // Cycle detected. Any cycle reachable from content context implies
                // the pattern eventually contains elements → Complex.
                return ContentType::Complex;
            }
            seen.insert(ptr);
            if let Some(rule) = pat_ref.0.borrow().as_ref() {
                content_type_impl(rule.pattern(), seen)
            } else {
                ContentType::Empty
            }
        }
        Pattern::DatatypeValue { .. } | Pattern::DatatypeName { .. } | Pattern::List(_) => {
            ContentType::Simple
        }
        Pattern::Attribute(_, _) => ContentType::Empty,
        Pattern::Group(members) | Pattern::Interleave(members) => {
            members
                .iter()
                .map(|m| content_type_impl(m, seen))
                .max()
                .unwrap_or(ContentType::Empty)
        }
        Pattern::Choice(alts) => {
            alts.iter()
                .filter(|a| !is_dead(a))
                .map(|a| content_type_impl(a, seen))
                .max()
                .unwrap_or(ContentType::Empty)
        }
        Pattern::OneOrMore(p) | Pattern::ZeroOrMore(p) | Pattern::Optional(p) => {
            content_type_impl(p, seen)
        }
        Pattern::Mixed(_) => ContentType::Complex, // mixed = interleave(text, ...) and text is complex
    }
}

fn groupable(ct1: ContentType, ct2: ContentType) -> bool {
    matches!(
        (ct1, ct2),
        (ContentType::Empty, _) | (_, ContentType::Empty) | (ContentType::Complex, ContentType::Complex)
    )
}

/// Check that all members of a group/interleave have groupable content types.
fn check_string_sequence(members: &[Pattern], span: codemap::Span) -> Result<(), RelaxError> {
    let types: Vec<ContentType> = members.iter()
        .filter(|m| !is_dead(m))
        .map(|m| content_type(m))
        .collect();
    for i in 0..types.len() {
        for j in (i + 1)..types.len() {
            if !groupable(types[i], types[j]) {
                return Err(restricted(span, "string sequence", "group/interleave"));
            }
        }
    }
    Ok(())
}

// --- Helper to construct a restriction error ---

fn restricted(span: codemap::Span, pattern_name: &str, context: &str) -> RelaxError {
    RelaxError::RestrictedPattern {
        span,
        pattern_name: pattern_name.to_string(),
        context: context.to_string(),
    }
}
