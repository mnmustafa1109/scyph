//! Extension traits for inline ABAC policy enforcement.
//!
//! Provides [`AuthUserEnforceExt`], which adds a fluent `.enforce(...)` method directly
//! onto [`scyph_auth::AuthUser`], enabling ergonomic single-line policy checks inside handler functions.
//!
//! # Design
//!
//! Rather than calling `DocumentPolicy::check(&user, &doc, action)` directly, this trait
//! lets you write `user.enforce::<DocumentPolicy>(&doc, Action::Read)?`, which reads more
//! naturally and mirrors the mental model of "the user is trying to do something to the resource".
//!
//! All enforcement calls are logged via `tracing`:
//! - **`DEBUG`** on policy pass: `user_id`, `action`
//! - **`WARN`**  on policy fail: `user_id`, `action`, `error`
//!
//! # Examples
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, AbacError, AuthUserEnforceExt};
//! use scyph_auth::AuthUser;
//! use scyph_core::{Action, AppError, ApiResponse, Claims};
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
//! struct Document { owner_id: Uuid }
//! struct DocumentPolicy;
//! impl AbacPolicy for DocumentPolicy {
//!     type Resource = Document;
//!     type Claims = MyClaims;
//!     fn check(user: &AuthUser<MyClaims>, doc: &Document, _: Action) -> Result<(), AbacError> {
//!         if doc.owner_id == user.id { Ok(()) }
//!         else { Err(AbacError::Forbidden("Not the document owner".into())) }
//!     }
//! }
//!
//! async fn get_document(
//!     user: AuthUser<MyClaims>,
//!     doc: Document,
//! ) -> Result<ApiResponse<String>, AppError> {
//!     // Fluent single-line enforcement — returns 403 Forbidden if check fails
//!     user.enforce::<DocumentPolicy>(&doc, Action::Read)?;
//!     Ok(ApiResponse::ok("document content".into()))
//! }
//!
//! let user_id = Uuid::now_v7();
//! let user = AuthUser {
//!     id: user_id,
//!     claims: MyClaims { sub: user_id, exp: 100, jti: "1".into(), role: Role::User },
//! };
//! let doc = Document { owner_id: user_id };
//! assert!(user.enforce::<DocumentPolicy>(&doc, Action::Read).is_ok());
//!
//! let other_doc = Document { owner_id: Uuid::now_v7() };
//! assert!(user.enforce::<DocumentPolicy>(&other_doc, Action::Read).is_err());
//! ```

use scyph_auth::AuthUser;
use scyph_core::{Action, Claims};

use crate::{AbacPolicy, error::AbacError};

/// Extension trait adding fluent `.enforce(...)` authorization methods to [`AuthUser`].
///
/// # Type Parameters
///
/// * `C` - Application JWT claims type implementing [`Claims`].
pub trait AuthUserEnforceExt<C: Claims> {
    /// Enforces an ABAC policy check for this user against a domain resource.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Policy type implementing [`AbacPolicy`] for claims `C`.
    ///
    /// # Arguments
    ///
    /// * `resource` - Reference to the target domain resource model.
    /// * `action` - The authorization action being attempted.
    ///
    /// # Errors
    ///
    /// Returns [`AbacError::Forbidden`] if the user is not permitted to perform the specified action.
    fn enforce<P: AbacPolicy<Claims = C>>(
        &self,
        resource: &P::Resource,
        action: Action,
    ) -> Result<(), AbacError>;
}

use tracing::{debug, warn};

impl<C: Claims> AuthUserEnforceExt<C> for AuthUser<C> {
    fn enforce<P: AbacPolicy<Claims = C>>(
        &self,
        resource: &P::Resource,
        action: Action,
    ) -> Result<(), AbacError> {
        match P::check(self, resource, action.clone()) {
            Ok(()) => {
                debug!(user_id = %self.id, action = ?action, "ABAC policy check passed");
                Ok(())
            }
            Err(err) => {
                warn!(user_id = %self.id, action = ?action, error = %err, "ABAC policy check failed");
                Err(err)
            }
        }
    }
}
