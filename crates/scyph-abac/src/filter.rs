//! SQLx query builder wrapper for dynamic ABAC SQL injection.
//!
//! Provides [`FilterBuilder`], a wrapper around [`sqlx::QueryBuilder`] for appending
//! parameterized SQL `WHERE` clauses safely without SQL injection vulnerabilities.
//!
//! # Design
//!
//! `FilterBuilder` wraps SQLx's `QueryBuilder<Postgres>` and exposes a chainable API
//! for constructing dynamic `WHERE` clauses driven by ABAC policy logic. The key distinction
//! from raw string concatenation:
//!
//! - **`push`**: Appends trusted static SQL fragments (column names, operators, SQL keywords).
//! - **`push_bind` / `push_uuid`**: Appends parameterized placeholders (`$1`, `$2`, …) and
//!   binds the actual value safely via the database driver, preventing SQL injection.
//!
//! # Usage Pattern
//!
//! `FilterBuilder` is typically used inside [`AbacPolicy::apply_query_filter`] implementations
//! to scope list queries to records the user is allowed to see:
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, AbacError, FilterBuilder};
//! use scyph_auth::AuthUser;
//! use scyph_core::Action;
//!
//! struct DocumentPolicy;
//! impl AbacPolicy for DocumentPolicy {
//!     type Resource = ();
//!     type Claims = AppClaims;
//!
//!     fn check(_: &AuthUser<AppClaims>, _: &(), _: Action) -> Result<(), AbacError> { Ok(()) }
//!
//!     fn apply_query_filter(filter: &mut FilterBuilder, subject: &AuthUser<AppClaims>, _: Action) {
//!         // Scope results: user sees public docs OR their own docs
//!         filter
//!             .push("(is_public = true OR owner_id = ")
//!             .push_uuid(subject.id)
//!             .push(")");
//!     }
//! }
//! ```
//!
//! Then in a handler:
//!
//! ```rust
//! use scyph_abac::FilterBuilder;
//! use uuid::Uuid;
//!
//! let user_id = Uuid::now_v7();
//! let mut filter = FilterBuilder::new("SELECT * FROM documents WHERE ");
//! filter.push("is_public = true OR owner_id = ").push_uuid(user_id);
//!
//! let sql_query = filter.into_inner().into_sql();
//! assert!(sql_query.as_str().contains("WHERE is_public = true OR owner_id = "));
//! ```
//!
//! # Security Note
//!
//! Never pass user-controlled input to [`FilterBuilder::push`]. Always use
//! [`FilterBuilder::push_bind`] or [`FilterBuilder::push_uuid`] for runtime values.

use scyph_auth::AuthUser;
use scyph_core::Action;
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::AbacPolicy;

/// Wrapper around [`sqlx::QueryBuilder`] for safe parameterized ABAC query filtering.
pub struct FilterBuilder(pub(crate) QueryBuilder<Postgres>);

impl FilterBuilder {
    /// Creates a new [`FilterBuilder`] initialized with a starting SQL query fragment string.
    ///
    /// # Arguments
    ///
    /// * `init` - Starting SQL string fragment (e.g. `"SELECT * FROM users WHERE "`).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use scyph_abac::FilterBuilder;
    ///
    /// let filter = FilterBuilder::new("SELECT * FROM users WHERE active = true");
    /// ```
    pub fn new(init: impl Into<String>) -> Self {
        Self(QueryBuilder::new(init.into()))
    }

    /// Appends a raw static SQL fragment string to the query.
    ///
    /// # Arguments
    ///
    /// * `sql` - Raw SQL string fragment to append.
    ///
    /// # Security Note
    ///
    /// Only pass trusted static SQL strings or column names to `push`. Never pass raw user inputs
    /// to `push` directly; use [`push_bind`](Self::push_bind) or [`push_uuid`](Self::push_uuid) instead.
    pub fn push(&mut self, sql: impl Into<String>) -> &mut Self {
        self.0.push(sql.into());
        self
    }

    /// Appends a parameterized value bind parameter to the query safely using prepared statements.
    ///
    /// # Arguments
    ///
    /// * `value` - Parameter value implementing SQL encoding for PostgreSQL.
    pub fn push_bind<'a, T>(&mut self, value: T) -> &mut Self
    where
        T: sqlx::Encode<'a, Postgres> + sqlx::Type<Postgres> + Send + 'a,
    {
        self.0.push_bind(value);
        self
    }

    /// Helper method to append a UUID parameter bind to the query.
    ///
    /// # Arguments
    ///
    /// * `id` - [`Uuid`] value to bind as a parameterized query variable.
    pub fn push_uuid(&mut self, id: Uuid) -> &mut Self {
        self.push_bind(id);
        self
    }

    /// Consumes the `FilterBuilder` and returns the inner [`sqlx::QueryBuilder`].
    ///
    /// Returns the underlying Postgres [`QueryBuilder`].
    pub fn into_inner(self) -> QueryBuilder<Postgres> {
        self.0
    }

    /// Applies an ABAC policy query filter for a given subject and action.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Type implementing [`AbacPolicy`].
    ///
    /// # Arguments
    ///
    /// * `subject` - Reference to the authenticated user [`AuthUser`].
    /// * `action` - The database query action being attempted.
    pub fn filter_by<P: AbacPolicy>(
        &mut self,
        subject: &AuthUser<P::Claims>,
        action: Action,
    ) -> &mut Self {
        P::apply_query_filter(self, subject, action);
        self
    }
}
