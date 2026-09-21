//! Trait and builder for creating Cedar entity records from domain models.

use cedar_policy::{Entity, EntityUid, RestrictedExpression};
use std::{collections::HashMap, str::FromStr};

use super::schema::CedarType;

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
