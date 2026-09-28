//! Attribute-Based Access Control (ABAC) policy trait definitions.
//!
//! Provides [`AbacPolicy`], the core interface for defining fine-grained authorization rules
//! and SQL query filter injections for application entities.
//!
//! # Examples
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, FilterBuilder};
//! use scyph_auth::AuthUser;
//! use scyph_core::{Action, AppError, Claims};
//! use uuid::Uuid;
//!
//! enum Role { User, Admin }
//!
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
//!     fn check(
//!         subject: &AuthUser<Self::Claims>,
//!         resource: &Self::Resource,
//!         action: Action,
//!     ) -> Result<(), AppError> {
//!         match action {
//!             Action::Read => {
//!                 if resource.is_public || resource.owner_id == subject.id || *subject.claims.role() == Role::Admin {
//!                     Ok(())
//!                 } else {
//!                     Err(AppError::Forbidden("Access denied".into()))
//!                 }
//!             }
//!             _ => Ok(()),
//!         }
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
    /// * `_alias` - Table alias identifier string (e.g., `"d"`).
    /// * `_action` - The database query action being attempted.
    fn apply_query_filter_with_alias(
        _filter: &mut FilterBuilder,
        _subject: &AuthUser<Self::Claims>,
        _alias: &str,
        _action: Action,
    ) {
    }
}
