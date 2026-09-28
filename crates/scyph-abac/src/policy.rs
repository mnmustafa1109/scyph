//! Attribute-Based Access Control (ABAC) policy trait definitions.
//!
//! Provides [`AbacPolicy`], the core interface for defining fine-grained authorization rules
//! and SQL query filter injections for application entities.
//!
//! # Design
//!
//! Each protected resource type in your domain gets its own policy struct. The policy struct
//! holds no state — it is a zero-sized type used purely as a namespace for associated functions.
//! This design keeps authorization logic co-located with the domain resource it guards.
//!
//! ```text
//! Document resource  →  DocumentPolicy  (implements AbacPolicy)
//! User resource      →  UserPolicy      (implements AbacPolicy)
//! Invoice resource   →  InvoicePolicy   (implements AbacPolicy)
//! ```
//!
//! # The Three Authorization Methods
//!
//! [`AbacPolicy`] provides three optional SQL filter hooks in addition to the required `check`:
//!
//! - [`check`](AbacPolicy::check): In-memory authorization against a loaded resource instance.
//!   Use this for single-resource operations (GET, UPDATE, DELETE).
//!
//! - [`apply_query_filter`](AbacPolicy::apply_query_filter): Appends SQL `WHERE` conditions
//!   for list queries where no specific resource instance exists yet.
//!   Use this for collection endpoints (LIST, SEARCH) to scope results to what the user can see.
//!
//! - [`apply_query_filter_with_resource`](AbacPolicy::apply_query_filter_with_resource):
//!   Appends SQL `WHERE` conditions using both user and resource attributes.
//!   Use this for sub-resource list queries (e.g., "list comments on this document").
//!
//! - [`apply_query_filter_with_alias`](AbacPolicy::apply_query_filter_with_alias):
//!   Like `apply_query_filter` but for JOIN queries using table aliases (e.g., `d.owner_id`).
//!
//! # Examples
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, AbacError, FilterBuilder};
//! use scyph_auth::AuthUser;
//! use scyph_core::{Action, Claims};
//! use uuid::Uuid;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
//! enum Role { User, Admin }
//!
//! #[derive(Clone, Serialize, Deserialize)]
//! struct MyClaims { sub: Uuid, exp: i64, jti: String, role: Role }
//! impl Claims for MyClaims {
//!     type Role = Role;
//!     fn subject(&self) -> Uuid { self.sub }
//!     fn expiry(&self) -> i64 { self.exp }
//!     fn jti(&self) -> &str { &self.jti }
//!     fn role(&self) -> &Self::Role { &self.role }
//! }
//!
//! struct Document { owner_id: Uuid, is_public: bool }
//! struct DocumentPolicy;
//!
//! impl AbacPolicy for DocumentPolicy {
//!     type Resource = Document;
//!     type Claims = MyClaims;
//!
//!     /// Enforce rules on a loaded document instance (GET / UPDATE / DELETE).
//!     fn check(
//!         subject: &AuthUser<Self::Claims>,
//!         resource: &Self::Resource,
//!         action: Action,
//!     ) -> Result<(), AbacError> {
//!         let is_owner = resource.owner_id == subject.id;
//!         let is_admin = *subject.claims.role() == Role::Admin;
//!
//!         match action {
//!             Action::Read => {
//!                 if resource.is_public || is_owner || is_admin {
//!                     Ok(())
//!                 } else {
//!                     Err(AbacError::Forbidden("Cannot read this document".into()))
//!                 }
//!             }
//!             Action::Update | Action::Delete => {
//!                 if is_owner || is_admin {
//!                     Ok(())
//!                 } else {
//!                     Err(AbacError::Forbidden("Cannot modify this document".into()))
//!                 }
//!             }
//!             _ => Ok(()),
//!         }
//!     }
//!
//!     /// Scope LIST queries so users only see documents they can access.
//!     fn apply_query_filter(
//!         filter: &mut FilterBuilder,
//!         subject: &AuthUser<Self::Claims>,
//!         _action: Action,
//!     ) {
//!         if *subject.claims.role() != Role::Admin {
//!             filter
//!                 .push("(is_public = true OR owner_id = ")
//!                 .push_uuid(subject.id)
//!                 .push(")");
//!         }
//!         // Admins see all documents — no filter appended
//!     }
//! }
//! ```

use crate::error::AbacError;
use crate::filter::FilterBuilder;
use scyph_auth::AuthUser;
use scyph_core::{Action, Claims};

/// Core ABAC policy trait for defining domain-level authorization rules and SQL filter injections.
///
/// Implement this trait on a policy struct for each protected domain entity (e.g. `DocumentPolicy`).
///
/// # Associated Types
///
/// * [`Resource`](Self::Resource) - The target domain entity model evaluated by this policy.
/// * [`Claims`](Self::Claims) - The application's custom JWT claims type implementing [`Claims`].
pub trait AbacPolicy {
    /// The target domain resource type evaluated by this policy (e.g. `Document`).
    type Resource: ?Sized;
    /// The application's custom JWT claims struct.
    type Claims: Claims;

    /// Evaluates in-memory authorization rules for a subject and resource.
    ///
    /// # Arguments
    ///
    /// * `subject` - Reference to the authenticated user [`AuthUser`].
    /// * `resource` - Reference to the target domain resource.
    /// * `action` - The authorization action being attempted.
    ///
    /// # Errors
    ///
    /// Returns [`AbacError::Forbidden`] if the subject is not permitted to perform the specified action.
    fn check(
        subject: &AuthUser<Self::Claims>,
        resource: &Self::Resource,
        action: Action,
    ) -> Result<(), AbacError>;

    /// Appends SQL `WHERE` conditions to a [`FilterBuilder`] based on subject attributes.
    ///
    /// Useful for database query filtering when querying lists of records without a specific resource instance.
    ///
    /// # Arguments
    ///
    /// * `_filter` - Mutable reference to the SQL [`FilterBuilder`].
    /// * `_subject` - Reference to the authenticated [`AuthUser`].
    /// * `_action` - The database query action being attempted.
    fn apply_query_filter(
        _filter: &mut FilterBuilder,
        _subject: &AuthUser<Self::Claims>,
        _action: Action,
    ) {
    }

    /// Appends SQL `WHERE` conditions to a [`FilterBuilder`] based on subject and resource attributes.
    ///
    /// Useful when filtering queries that depend on specific target resource attributes.
    ///
    /// # Arguments
    ///
    /// * `_filter` - Mutable reference to the SQL [`FilterBuilder`].
    /// * `_subject` - Reference to the authenticated [`AuthUser`].
    /// * `_resource` - Reference to the target domain resource instance.
    /// * `_action` - The database query action being attempted.
    fn apply_query_filter_with_resource(
        _filter: &mut FilterBuilder,
        _subject: &AuthUser<Self::Claims>,
        _resource: &Self::Resource,
        _action: Action,
    ) {
    }

    /// Appends SQL `WHERE` conditions to a [`FilterBuilder`] using a table alias identifier.
    ///
    /// Useful when the SQL query involves JOINs or table aliases (e.g., `d.owner_id`).
    ///
    /// # Arguments
    ///
    /// * `_filter` - Mutable reference to the SQL [`FilterBuilder`].
    /// * `_subject` - Reference to the authenticated [`AuthUser`].
    /// * `_action` - The database query action being attempted.
    fn apply_query_filter_with_alias(
        _filter: &mut FilterBuilder,
        _subject: &AuthUser<Self::Claims>,
        _action: Action,
    ) {
    }
}
