//! Round-trip and validation tests for the Cedar schema builder and
//! [`CedarAuthorizer::from_str`] startup validation.
//!
//! Guards three silent failure modes that no linter can catch:
//! 1. the emitted schema DSL must parse (cedar-policy spells the primitive
//!    `Bool`; emitting `Boolean` makes every schema with an `is_active`-style
//!    attribute fail at startup),
//! 2. custom actions must not collide with (or duplicate) the built-in CRUD
//!    actions,
//! 3. policies that reference unknown attributes/entities must fail loudly at
//!    construction (`PolicyValidation`), not deny requests at runtime.

#![cfg(feature = "cedar")]

use scyph_abac::{CedarAuthorizer, CedarError, CedarSchemaBuilder, CedarType};
use std::str::FromStr;

const POLICY_WITH_BOOL: &str =
    r#"permit(principal, action == Action::"Update", resource) when { resource.active == true };"#;

fn schema_all_types() -> String {
    let mut builder = CedarSchemaBuilder::new();
    builder
        .add_attribute("Document", "title", CedarType::String)
        .add_attribute("Document", "active", CedarType::Bool)
        .add_attribute("Document", "owner", CedarType::EntityRef("User"));
    builder
        .add_action("Deactivate")
        .add_action("Read") // built-in collision: must be ignored
        .add_action("Deactivate") // duplicate: must be ignored
        .build()
}

/// `build()` output must parse AND validate against a policy using every
/// emitted feature: String, Boolean (the `Bool` regression), EntityRef, and a
/// custom action.
#[test]
fn built_schema_round_trips_and_validates() {
    cedar_policy::Schema::from_str(&schema_all_types()).expect("schema DSL should parse");
    let result = CedarAuthorizer::from_str(POLICY_WITH_BOOL, Some(&schema_all_types()));
    assert!(result.is_ok(), "policy+schema should validate");
}

/// `for_principal("Actor")` must emit `principal: Actor` (never `principal: User`,
/// which would break every request for an Actor principal) and the schema — with
/// no `User` entity present — must still parse and statically validate a policy.
#[test]
fn custom_principal_round_trips_and_validates() {
    let mut builder = CedarSchemaBuilder::for_principal("Actor");
    builder
        .add_attribute("Document", "title", CedarType::String)
        .add_attribute("Document", "owner", CedarType::EntityRef("Actor"));
    let schema = builder.add_action("Deactivate").build();
    assert!(
        schema.contains("principal: Actor"),
        "wrong principal: {schema}"
    );
    assert!(!schema.contains("principal: User"), "stray User: {schema}");
    cedar_policy::Schema::from_str(&schema).expect("custom-principal schema should parse");
    assert!(
        CedarAuthorizer::from_str("permit(principal, action, resource);", Some(&schema)).is_ok(),
        "policy with Actor principal should validate"
    );
}

/// A policy referencing an attribute that does not exist in the schema fails
/// at startup with `PolicyValidation`, not at request time.
#[test]
fn unknown_attribute_fails_startup_validation() {
    let bad = r#"permit(principal, action == Action::"Update", resource) when { resource.missing_attr == 1 };"#;
    match CedarAuthorizer::from_str(bad, Some(&schema_all_types())) {
        Err(CedarError::PolicyValidation(msg)) => assert!(
            msg.contains("missing_attr"),
            "error should name the offending attribute: {msg}"
        ),
        Ok(_) => panic!("expected PolicyValidation, got Ok"),
        Err(e) => panic!("expected PolicyValidation, got {e:?}"),
    }
}

/// A source that parses as neither Cedar DSL nor JSON yields `SchemaParse`.
#[test]
fn unparseable_schema_reports_schema_parse() {
    match CedarAuthorizer::from_str(POLICY_WITH_BOOL, Some("this is not a schema")) {
        Err(CedarError::SchemaParse(msg)) => {
            assert!(
                msg.contains("as Cedar schema DSL") && msg.contains("as JSON"),
                "error should explain both parse attempts: {msg}"
            );
        }
        Ok(_) => panic!("expected SchemaParse, got Ok"),
        Err(e) => panic!("expected SchemaParse, got {e:?}"),
    }
}

/// Malformed policy source still reports `PolicyParse` before any schema work.
#[test]
fn malformed_policy_reports_policy_parse() {
    match CedarAuthorizer::from_str("permit(", Some(&schema_all_types())) {
        Err(CedarError::PolicyParse(_)) => {}
        Ok(_) => panic!("expected PolicyParse, got Ok"),
        Err(e) => panic!("expected PolicyParse, got {e:?}"),
    }
}

/// No schema provided: pure parse, no validation — still must succeed.
#[test]
fn without_schema_policy_only_parses() {
    assert!(
        CedarAuthorizer::from_str(POLICY_WITH_BOOL, None).is_ok(),
        "policy parse without schema should succeed"
    );
}
