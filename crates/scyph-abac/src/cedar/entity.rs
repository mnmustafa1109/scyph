//! Trait and builder for creating Cedar entity records from domain models.

use cedar_policy::{Entity, EntityUid, RestrictedExpression};
use std::{collections::HashMap, str::FromStr};

use super::error::CedarError;
use super::schema::CedarType;

/// Trait implemented by application models to convert struct records into Cedar Entities.
pub trait IntoCedarEntity {
    /// Cedar entity type identifier string (e.g. `"Document"`).
    fn entity_type() -> &'static str;
    /// Unique identifier for this entity instance.
    fn entity_id(&self) -> String;
    /// Converts this record instance into a Cedar [`Entity`].
    fn to_cedar_entity(&self) -> Result<Entity, CedarError>;
    /// Returns the attribute types for this entity.
    fn attribute_types() -> HashMap<String, CedarType>;

    /// Returns the unique Cedar [`EntityUid`] for this record.
    fn to_entity_uid(&self) -> Result<EntityUid, CedarError> {
        EntityUid::from_str(&format!(
            "{}::\"{}\"",
            Self::entity_type(),
            self.entity_id()
        ))
        .map_err(|e| CedarError::RequestBuild(e.to_string()))
    }
}

/// Builder for constructing Cedar [`Entity`] records with typed attributes.
pub struct CedarEntityBuilder {
    uid: Option<EntityUid>,
    attrs: HashMap<String, RestrictedExpression>,
    error: Option<CedarError>,
}

impl CedarEntityBuilder {
    /// Initializes a [`CedarEntityBuilder`] for an entity type and record ID.
    ///
    /// # Arguments
    ///
    /// * `entity_type` - Cedar entity type identifier string (e.g., `"Document"`).
    /// * `entity_id` - Entity record identifier string.
    pub fn new(entity_type: &str, entity_id: &str) -> Self {
        match EntityUid::from_str(&format!("{}::\"{}\"", entity_type, entity_id)) {
            Ok(uid) => Self {
                uid: Some(uid),
                attrs: HashMap::new(),
                error: None,
            },
            Err(e) => Self {
                uid: None,
                attrs: HashMap::new(),
                error: Some(CedarError::RequestBuild(e.to_string())),
            },
        }
    }

    /// Appends a boolean attribute.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name.
    /// * `val` - Boolean value.
    pub fn attr_bool(mut self, key: impl Into<String>, val: bool) -> Self {
        if self.error.is_none() {
            self.attrs
                .insert(key.into(), RestrictedExpression::new_bool(val));
        }
        self
    }

    /// Appends a string attribute.
    ///
    /// # Arguments
    ///
    /// * `key` - Attribute name.
    /// * `val` - String value.
    pub fn attr_string(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        if self.error.is_none() {
            self.attrs
                .insert(key.into(), RestrictedExpression::new_string(val.into()));
        }
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
        if self.error.is_some() {
            return self;
        }
        match EntityUid::from_str(&format!("{}::\"{}\"", target_type, target_id)) {
            Ok(target_uid) => {
                self.attrs.insert(
                    key.into(),
                    RestrictedExpression::new_entity_uid(target_uid),
                );
            }
            Err(e) => {
                self.error = Some(CedarError::RequestBuild(e.to_string()));
            }
        }
        self
    }

    /// Builds the Cedar [`Entity`].
    pub fn build(self) -> Result<Entity, CedarError> {
        if let Some(err) = self.error {
            return Err(err);
        }
        let uid = self
            .uid
            .ok_or_else(|| CedarError::RequestBuild("missing entity uid".into()))?;
        Entity::new(uid, self.attrs, std::collections::HashSet::new())
            .map_err(|e| CedarError::RequestBuild(e.to_string()))
    }
}
