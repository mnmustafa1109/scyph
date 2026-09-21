//! Cedar policy evaluator implementation.

use cedar_policy::{
    Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request, Schema,
};
use scyph_auth::AuthUser;
use scyph_core::{Action, AppError, Claims};
use std::str::FromStr;
use tracing::{debug, warn};

use super::{entity::IntoCedarEntity, error::CedarError};

/// Evaluator wrapper for executing AWS Cedar policy rules against application entities.
pub struct CedarAuthorizer {
    authorizer: Authorizer,
    policies: PolicySet,
    schema: Option<Schema>,
}

impl CedarAuthorizer {
    /// Constructs a [`CedarAuthorizer`] by parsing Cedar policy source text and optional schema text.
    ///
    /// # Arguments
    ///
    /// * `policy_src` - Cedar policy language source string.
    /// * `schema_src` - Optional Cedar schema source string for request validation.
    ///
    /// # Errors
    ///
    /// Returns [`CedarError::PolicyParse`] or [`CedarError::SchemaParse`] if syntax validation fails.
    pub fn from_str(policy_src: &str, schema_src: Option<&str>) -> Result<Self, CedarError> {
        let policies =
            PolicySet::from_str(policy_src).map_err(|e| CedarError::PolicyParse(e.to_string()))?;
        let schema = schema_src
            .map(|s| Schema::from_str(s).map_err(|e| CedarError::SchemaParse(e.to_string())))
            .transpose()?;
        let authorizer = Authorizer::new();
        Ok(Self {
            authorizer,
            policies,
            schema,
        })
    }

    /// Evaluates raw Cedar [`Request`] components and returns `Ok(())` if allowed or [`CedarError::AccessDenied`] if denied.
    ///
    /// # Arguments
    ///
    /// * `principal` - Entity UID of the evaluating user/principal.
    /// * `action` - Entity UID of the action being requested.
    /// * `resource` - Entity UID of the target resource.
    /// * `context` - Cedar [`Context`] containing optional request headers/attributes.
    /// * `entities` - Container [`Entities`] holding involved entity records.
    ///
    /// # Errors
    ///
    /// Returns [`CedarError::AccessDenied`] if decision is `Deny` or [`CedarError::RequestBuild`] if request generation fails.
    pub fn is_authorized(
        &self,
        principal: &EntityUid,
        action: &EntityUid,
        resource: &EntityUid,
        context: Context,
        entities: &Entities,
    ) -> Result<(), CedarError> {
        let request = Request::new(
            principal.clone(),
            action.clone(),
            resource.clone(),
            context,
            self.schema.as_ref(),
        )
        .map_err(|e| CedarError::RequestBuild(e.to_string()))?;
        let response = self
            .authorizer
            .is_authorized(&request, &self.policies, entities);
        match response.decision() {
            Decision::Allow => {
                debug!(
                    principal = %principal,
                    action = %action,
                    resource = %resource,
                    "Cedar authorization decision: ALLOW"
                );
                Ok(())
            }
            Decision::Deny => {
                warn!(
                    principal = %principal,
                    action = %action,
                    resource = %resource,
                    "Cedar authorization decision: DENY"
                );
                Err(CedarError::AccessDenied)
            }
        }
    }

    /// Evaluates authorization for a model implementing [`IntoCedarEntity`].
    ///
    /// # Type Parameters
    ///
    /// * `R` - Resource type implementing [`IntoCedarEntity`].
    ///
    /// # Arguments
    ///
    /// * `principal_type` - Cedar entity type for the user (e.g. `"User"`).
    /// * `principal_id` - Identifier string of the user.
    /// * `action_name` - Cedar action name (e.g. `"Read"`).
    /// * `resource` - Reference to a resource model implementing [`IntoCedarEntity`].
    ///
    /// # Errors
    ///
    /// Returns [`AppError::BadRequest`] if UIDs are invalid, or [`AppError::Forbidden`] if denied.
    pub fn authorize<R: IntoCedarEntity>(
        &self,
        principal_type: &str,
        principal_id: &str,
        action_name: &str,
        resource: &R,
    ) -> Result<(), AppError> {
        let principal = EntityUid::from_str(&format!("{}::\"{}\"", principal_type, principal_id))
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        let action = EntityUid::from_str(&format!("Action::\"{}\"", action_name))
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let resource_uid = resource.to_entity_uid();
        let entities =
            Entities::from_entities(vec![resource.to_cedar_entity()], self.schema.as_ref())
                .map_err(|e| AppError::Internal {
                    source: Box::new(e),
                    context: "Entities build failed".into(),
                })?;

        self.is_authorized(
            &principal,
            &action,
            &resource_uid,
            Context::empty(),
            &entities,
        )?;
        Ok(())
    }

    /// Strongly-typed Cedar authorization check using [`AuthUser`], [`Action`], and resource model.
    ///
    /// # Type Parameters
    ///
    /// * `C` - Application claims type implementing [`Claims`].
    /// * `R` - Resource model implementing [`IntoCedarEntity`].
    ///
    /// # Arguments
    ///
    /// * `user` - Reference to the authenticated [`AuthUser`].
    /// * `action` - The [`Action`] enum variant.
    /// * `resource` - Reference to a resource model implementing [`IntoCedarEntity`].
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Forbidden`] if denied, or [`AppError::BadRequest`] if action mapping fails.
    pub fn check<C: Claims, R: IntoCedarEntity>(
        &self,
        user: &AuthUser<C>,
        action: Action,
        resource: &R,
    ) -> Result<(), AppError> {
        let action_name = match &action {
            Action::Create => "Create",
            Action::Read => "Read",
            Action::Update => "Update",
            Action::Delete => "Delete",
            Action::List => "List",
            Action::Custom(name) => name.as_ref(),
            &_ => {
                return Err(AppError::BadRequest(format!(
                    "Unsupported action: {:?}",
                    action
                )));
            }
        };

        self.authorize("User", &user.id.to_string(), action_name, resource)
    }
}
