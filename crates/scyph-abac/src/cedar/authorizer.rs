//! Cedar policy evaluator implementation.

use cedar_policy::{
    Authorizer, Context, Decision, Entities, EntityUid, PolicySet, Request, Schema, ValidationMode,
    Validator,
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
    /// When a schema is provided, the policy set is statically validated against it
    /// ([`CedarError::PolicyValidation`] on failure) and warnings are logged.
    pub fn from_str(policy_src: &str, schema_src: Option<&str>) -> Result<Self, CedarError> {
        let policies =
            PolicySet::from_str(policy_src).map_err(|e| CedarError::PolicyParse(e.to_string()))?;
        let schema = schema_src
            .map(|s| {
                // Accept both formats: hand-written Cedar schema DSL
                // (`from_str`) and JSON (`from_json_str`).
                Schema::from_str(s).or_else(|dsl_err| {
                    Schema::from_json_str(s).map_err(|json_err| {
                        CedarError::SchemaParse(format!(
                            "as Cedar schema DSL: {dsl_err}; as JSON: {json_err}"
                        ))
                    })
                })
            })
            .transpose()?;
        if let Some(schema) = &schema {
            let result = Validator::new(schema.clone()).validate(&policies, ValidationMode::Strict);
            for warning in result.validation_warnings() {
                warn!("Cedar policy validation warning: {warning}");
            }
            let errors: Vec<String> = result.validation_errors().map(|e| e.to_string()).collect();
            if !errors.is_empty() {
                return Err(CedarError::PolicyValidation(errors.join("; ")));
            }
        }
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

    /// Evaluates authorization for a principal model `P` and resource model `R`, both implementing [`IntoCedarEntity`].
    ///
    /// Loads both principal and resource entity attributes into Cedar's evaluation context.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Principal entity model type implementing [`IntoCedarEntity`].
    /// * `R` - Resource entity model type implementing [`IntoCedarEntity`].
    ///
    /// # Arguments
    ///
    /// * `principal` - Reference to the evaluating principal entity model (e.g. `User`).
    /// * `action_name` - Cedar action name string (e.g. `"Read"`).
    /// * `resource` - Reference to the target resource entity model (e.g. `Document`).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::BadRequest`] if UIDs are invalid, or [`AppError::Forbidden`] if denied.
    pub fn authorize<P: IntoCedarEntity, R: IntoCedarEntity>(
        &self,
        principal: &P,
        action_name: &str,
        resource: &R,
    ) -> Result<(), AppError> {
        let principal_uid = principal.to_entity_uid()?;
        let resource_uid = resource.to_entity_uid()?;
        let action = EntityUid::from_str(&format!("Action::\"{}\"", action_name))
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        let entities = Entities::from_entities(
            vec![principal.to_cedar_entity()?, resource.to_cedar_entity()?],
            self.schema.as_ref(),
        )
        .map_err(|e| AppError::Internal {
            source: Box::new(e),
            context: "Entities build failed".into(),
        })?;

        self.is_authorized(
            &principal_uid,
            &action,
            &resource_uid,
            Context::empty(),
            &entities,
        )?;
        Ok(())
    }

    /// Evaluates authorization using string UIDs for principal type and ID when a full principal entity model is unavailable.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Resource entity model type implementing [`IntoCedarEntity`].
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
    pub fn authorize_uids<R: IntoCedarEntity>(
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

        let resource_uid = resource.to_entity_uid()?;
        let entities =
            Entities::from_entities(vec![resource.to_cedar_entity()?], self.schema.as_ref())
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

    /// Strongly-typed Cedar authorization check using principal model `P`, [`Action`], and resource model `R`.
    ///
    /// # Type Parameters
    ///
    /// * `P` - Principal entity model implementing [`IntoCedarEntity`].
    /// * `R` - Resource entity model implementing [`IntoCedarEntity`].
    ///
    /// # Arguments
    ///
    /// * `principal` - Reference to the principal entity model (e.g. `User`).
    /// * `action` - The [`Action`] enum variant.
    /// * `resource` - Reference to a resource model implementing [`IntoCedarEntity`].
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Forbidden`] if denied, or [`AppError::BadRequest`] if action mapping fails.
    pub fn check<P: IntoCedarEntity, R: IntoCedarEntity>(
        &self,
        principal: &P,
        action: Action,
        resource: &R,
    ) -> Result<(), AppError> {
        let action_name = action_label(&action)?;
        self.authorize(principal, action_name, resource)
    }

    /// Strongly-typed Cedar authorization check using [`AuthUser`], [`Action`], and resource model.
    ///
    /// Evaluates using string principal UID `"User::<user_id>"`.
    pub fn check_user<C: Claims, R: IntoCedarEntity>(
        &self,
        user: &AuthUser<C>,
        action: Action,
        resource: &R,
    ) -> Result<(), AppError> {
        let action_name = action_label(&action)?;
        self.authorize_uids("User", &user.id.to_string(), action_name, resource)
    }
}

/// Maps an [`Action`] to its Cedar action name (kept as one table for `check`/`check_user`).
///
/// `Action` is `#[non_exhaustive]`, so downstream additions hit this single arm instead of
/// two duplicated matches going out of sync.
fn action_label(action: &Action) -> Result<&str, AppError> {
    match action {
        Action::Create => Ok("Create"),
        Action::Read => Ok("Read"),
        Action::Update => Ok("Update"),
        Action::Delete => Ok("Delete"),
        Action::List => Ok("List"),
        Action::Custom(name) => Ok(name.as_ref()),
        _ => Err(AppError::BadRequest(format!(
            "Unsupported action: {action:?}"
        ))),
    }
}
