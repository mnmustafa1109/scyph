// crates/skidblad-auth/src/rbac.rs
use crate::extractor::AuthUser;
use skidblad_core::error::AppError;
use skidblad_core::traits::Claims;

pub fn require_role<C: Claims, F>(user: &AuthUser<C>, predicate: F) -> Result<(), AppError>
where
    F: Fn(&C) -> bool,
{
    if predicate(&user.claims) {
        Ok(())
    } else {
        Err(AppError::Forbidden("Insufficient role".into()))
    }
}
