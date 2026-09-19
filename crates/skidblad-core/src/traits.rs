// crates/skidblad-core/src/traits.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait Authorizable: Copy + Eq + Send + Sync + 'static {}
impl<T> Authorizable for T where T: Copy + Eq + Send + Sync + 'static {}

/// Implement this on your project's JWT claims struct.
///
/// The type parameter on `AuthUser<C>` (step 03) is `C: Claims`.
/// Your project defines exactly one claims struct and one impl.
///
/// ```rust
/// use skidblad_core::Claims;
/// use uuid::Uuid;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Clone, Serialize, Deserialize)]
/// struct MyClaims { sub: Uuid, exp: i64, iat: i64, jti: String }
///
/// impl Claims for MyClaims {
///     fn subject(&self) -> Uuid { self.sub }
///     fn expiry(&self) -> i64  { self.exp }
///     fn jti(&self) -> &str    { &self.jti }
/// }
/// ```
pub trait Claims: serde::de::DeserializeOwned + Serialize + Send + Sync + Clone + 'static {
    type Role: Authorizable;

    fn subject(&self) -> Uuid;
    fn expiry(&self) -> i64;
    fn jti(&self) -> &str;
    fn role(&self) -> &Self::Role;

    /// Returns true if the token's expiry is in the past.
    /// Defaults to UTC now > exp. Override if you use a non-Unix epoch.
    fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expiry()
    }
}

/// Authorization action used by `AbacPolicy` (step 04).
///
/// The `#[non_exhaustive]` attribute lets downstream crates extend
/// via `Custom("my_action")` without breaking `match` arms in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Action {
    Create,
    Read,
    Update,
    Delete,
    List,
    Custom(&'static str),
}
