//! Programmatic Cedar JSON schema builder and type definitions.

use serde_json::json;
use std::collections::HashMap;

use super::entity::IntoCedarEntity;

/// Represents supported data types for entity attributes when building Cedar JSON schemas.
pub enum CedarType {
    /// Boolean primitive type (`true` / `false`).
    Boolean,
    /// String primitive type.
    String,
    /// Reference to another Cedar entity type.
    EntityRef(&'static str),
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
