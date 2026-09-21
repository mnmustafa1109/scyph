//! AWS Cedar Policy Language engine integration for dynamic policy evaluation.
//!
//! Provides [`CedarAuthorizer`] to evaluate authorization rules written in Cedar policy language (`.cedar`),
//! [`IntoCedarEntity`] for converting database models into Cedar entities, and [`CedarEntityBuilder`].
//!
//! # Examples
//!
//! ```rust
//! use scyph_abac::cedar::{CedarAuthorizer, CedarEntityBuilder, CedarType, IntoCedarEntity};
//! use cedar_policy::Entity;
//! use std::collections::HashMap;
//! use uuid::Uuid;
//!
//! pub struct Document { pub id: Uuid, pub is_public: bool }
//! impl IntoCedarEntity for Document {
//!     fn entity_type() -> &'static str { "Document" }
//!     fn entity_id(&self) -> String { self.id.to_string() }
//!     fn to_cedar_entity(&self) -> Entity {
//!         CedarEntityBuilder::new("Document", &self.id.to_string())
//!             .attr_bool("is_public", self.is_public)
//!             .build()
//!     }
//!     fn attribute_types() -> HashMap<String, CedarType> {
//!         let mut m = HashMap::new();
//!         m.insert("is_public".to_string(), CedarType::Boolean);
//!         m
//!     }
//! }
//!
//! let policy_src = r#"
//!     permit (
//!         principal,
//!         action == Action::"Read",
//!         resource
//!     )
//!     when { resource.is_public == true };
//! "#;
//!
//! let authorizer = CedarAuthorizer::from_str(policy_src, None).unwrap();
//! let doc = Document { id: Uuid::new_v4(), is_public: true };
//!
//! assert!(authorizer.authorize("User", "alice", "Read", &doc).is_ok());
//! ```

use cedar_policy::{
    Authorizer, Context, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    RestrictedExpression, Schema,
};
use scyph_auth::AuthUser;
use scyph_core::{Action, AppError, Claims};
use serde_json::json;
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;

/// Errors encountered during Cedar policy parsing, schema validation, or request evaluation.
#[derive(Debug, Error)]
pub enum CedarError {
    /// Failed to parse Cedar policy source text.
    #[error("Policy parse error: {0}")]
    PolicyParse(String),

    /// Failed to parse Cedar schema text.
    #[error("Schema parse error: {0}")]
    SchemaParse(String),

    /// Failed to construct Cedar request or entity container.
    #[error("Request build error: {0}")]
    RequestBuild(String),

    /// Evaluation error encountered during authorization.
    #[error("Evaluation error: {0}")]
    Evaluation(String),

    /// Authorization evaluation resulted in `Decision::Deny`.
    #[error("Access denied")]
    AccessDenied,
}

/// Evaluator wrapper for executing AWS Cedar policy rules against application entities.
pub struct CedarAuthorizer {
    authorizer: Authorizer,
    policies: PolicySet,
    schema: Option<Schema>,
}

/// Represents supported data types for entity attributes when building Cedar JSON schemas.
pub enum CedarType {
    /// Boolean primitive type (`true` / `false`).
    Boolean,
    /// String primitive type.
    String,
    /// Reference to another Cedar entity type.
    EntityRef(&'static str),
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
            Decision::Allow => Ok(()),
            Decision::Deny => Err(CedarError::AccessDenied),
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

impl From<CedarError> for AppError {
    fn from(e: CedarError) -> Self {
        match e {
            CedarError::AccessDenied => AppError::Forbidden("Not Permitted".into()),
            other => AppError::internal_from(other, "Cedar authorization error"),
        }
    }
}

/// Trait implemented by application models to convert struct records into Cedar Entities.
pub trait IntoCedarEntity {
    /// Cedar entity type identifier string (e.g. `"Document"`).
    fn entity_type() -> &'static str;
    /// Unique identifier for this entity instance.
    fn entity_id(&self) -> String;
    /// Converts this record instance into a Cedar [`Entity`].
    fn to_cedar_entity(&self) -> Entity;
    /// Returns the attribute types for this entity.
    fn attribute_types() -> HashMap<String, CedarType>;

    /// Returns the unique Cedar [`EntityUid`] for this record.
    fn to_entity_uid(&self) -> EntityUid {
        EntityUid::from_str(&format!(
            "{}::\"{}\"",
            Self::entity_type(),
            self.entity_id()
        ))
        .expect("Valid Cedar EntityUid string")
    }
}

/// Builder for constructing Cedar [`Entity`] records with typed attributes.
pub struct CedarEntityBuilder {
    uid: EntityUid,
    attrs: HashMap<String, RestrictedExpression>,
}

impl CedarEntityBuilder {
    /// Initializes a [`CedarEntityBuilder`] for an entity type and record ID.
    ///
    /// # Arguments
    ///
    /// * `entity_type` - Cedar entity type identifier string (e.g., `"Document"`).
    /// * `entity_id` - Entity record identifier string.
    pub fn new(entity_type: &str, entity_id: &str) -> Self {
        let uid = EntityUid::from_str(&format!("{}::\"{}\"", entity_type, entity_id))
            .expect("Valid Cedar EntityUid string");
        Self {
            uid,
            attrs: HashMap::new(),
        }
    }

    /// Appends a boolean attribute.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name.
    /// * `val` - Boolean value.
    pub fn attr_bool(mut self, key: impl Into<String>, val: bool) -> Self {
        self.attrs
            .insert(key.into(), RestrictedExpression::new_bool(val));
        self
    }

    /// Appends a string attribute.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name.
    /// * `val` - String value.
    pub fn attr_string(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.attrs
            .insert(key.into(), RestrictedExpression::new_string(val.into()));
        self
    }

    /// Appends an entity reference attribute (e.g. `owner -> User::"uuid"`).
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name.
    /// * `target_type` - Entity type of the target record (e.g., `"User"`).
    /// * `target_id` - Identifier of the target record.
    pub fn attr_entity_ref(
        mut self,
        key: impl Into<String>,
        target_type: &str,
        target_id: &str,
    ) -> Self {
        let target_uid = EntityUid::from_str(&format!("{}::\"{}\"", target_type, target_id))
            .unwrap_or_else(|_| {
                panic!(
                    "Invalid target entity type/id: {}::{}",
                    target_type, target_id
                )
            });
        self.attrs
            .insert(key.into(), RestrictedExpression::new_entity_uid(target_uid));
        self
    }

    /// Builds the Cedar [`Entity`].
    ///
    /// Returns the completed [`cedar_policy::Entity`] instance.
    pub fn build(self) -> Entity {
        Entity::new(self.uid, self.attrs, std::collections::HashSet::new())
            .expect("Valid Cedar Entity")
    }
}

/// Builder for constructing Cedar JSON schemas programmatically from application entities.
///
/// Enables defining entity types and their attributes to generate JSON schema definitions
/// compatible with the AWS Cedar policy engine schema parser.
pub struct CedarSchemaBuilder {
    entity_types: HashMap<String, HashMap<String, CedarType>>,
}

impl Default for CedarSchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CedarSchemaBuilder {
    /// Creates a new [`CedarSchemaBuilder`] pre-populated with a base `User` entity type.
    pub fn new() -> Self {
        let mut entity_types = HashMap::new();
        entity_types.insert("User".into(), HashMap::new());
        Self { entity_types }
    }

    /// Registers entity attribute definitions from a type implementing [`IntoCedarEntity`].
    ///
    /// # Type Parameters
    ///
    /// * `E` - Entity type implementing [`IntoCedarEntity`].
    pub fn register_entity<E: IntoCedarEntity>(mut self) -> Self {
        let entity_name = E::entity_type().to_string();
        let attrs_map = self.entity_types.entry(entity_name.clone()).or_default();
        attrs_map.extend(E::attribute_types());
        self
    }

    /// Manually adds or updates an attribute definition for a given entity type.
    ///
    /// # Arguments
    ///
    /// * `entity_type` - Name of the entity type (e.g., `"Document"`).
    /// * `attr_name` - Name of the attribute.
    /// * `attr_type` - [`CedarType`] representing the data type of the attribute.
    pub fn add_attribute(
        &mut self,
        entity_type: &str,
        attr_name: &str,
        attr_type: CedarType,
    ) -> &mut Self {
        self.entity_types
            .entry(entity_type.to_string())
            .or_default()
            .insert(attr_name.to_string(), attr_type);
        self
    }

    /// Builds and exports the Cedar schema as a formatted JSON string.
    ///
    /// Returns a JSON string suitable for parsing into [`cedar_policy::Schema`].
    pub fn build_json_schema(&self) -> String {
        let mut entity_json_map = serde_json::Map::new();
        for (entity_name, attrs) in &self.entity_types {
            let mut shape_attrs = serde_json::Map::new();
            for (attr_name, attr_type) in attrs {
                let type_def = match attr_type {
                    CedarType::Boolean => json!({ "type": "Boolean" }),
                    CedarType::String => json!({ "type": "String" }),
                    CedarType::EntityRef(target) => json!({
                                "type": "Entity",
                                "name": target
                    }),
                };
                shape_attrs.insert(attr_name.clone(), type_def);
            }
            let entity_json = json!({
                "memberOfTypes": [],
                "shape": {
                    "type": "Record",
                    "attributes": shape_attrs
                } });

            entity_json_map.insert(entity_name.clone(), entity_json);
        }

        json!({
            "": {
                "entityTypes": entity_json_map,
                "actions": {
                    "Read": { "appliesTo": { "principalTypes": ["User"], "resourceTypes": null } },
                    "Create": { "appliesTo": { "principalTypes": ["User"], "resourceTypes": null } },
                    "Update": { "appliesTo": { "principalTypes": ["User"], "resourceTypes": null } },
                    "Delete": { "appliesTo": { "principalTypes": ["User"], "resourceTypes": null }    }
                }
            }
        })
        .to_string()
    }
}
