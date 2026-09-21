//! Extension traits for inline ABAC policy enforcement.
//!
//! Provides [`AuthUserEnforceExt`], which adds `.enforce(...)` directly onto [`AuthUser`].
//!
//! # Examples
//!
//! ```rust,ignore
//! use scyph_abac::{AbacPolicy, AuthUserEnforceExt, FilterBuilder};
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
//! struct Document { owner_id: Uuid }
//! struct DocumentPolicy;
//! impl AbacPolicy for DocumentPolicy {
//!     type Resource = Document;
//!     type Claims = MyClaims;
//!     fn check(user: &AuthUser<MyClaims>, doc: &Document, action: Action) -> Result<(), AppError> {
//!         if doc.owner_id == user.id { Ok(()) } else { Err(AppError::Forbidden("Denied".into())) }
//!     }
//! }
//!
//! let user_id = Uuid::new_v4();
//! let user = AuthUser { id: user_id, claims: MyClaims { sub: user_id, exp: 100, jti: "1".into(), role: Role::User } };
//! let doc = Document { owner_id: user_id };
//!
//! assert!(user.enforce::<DocumentPolicy>(&doc, Action::Read).is_ok());
//! ```

use scyph_auth::AuthUser;
use scyph_core::{Action, AppError, Claims};

use crate::AbacPolicy;

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
    /// Returns [`AppError::Forbidden`] if the user is not permitted to perform the specified action.
    fn enforce<P: AbacPolicy<Claims = C>>(
        &self,
        resource: &P::Resource,
        action: Action,
    ) -> Result<(), AppError>;
}

impl<C: Claims> AuthUserEnforceExt<C> for AuthUser<C> {
    fn enforce<P: AbacPolicy<Claims = C>>(
        &self,
        resource: &P::Resource,
        action: Action,
    ) -> Result<(), AppError> {
        P::check(self, resource, action)
    }
}
