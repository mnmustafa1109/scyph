//! Programmatic Cedar schema builder and type definitions.

use std::collections::HashMap;

use super::entity::IntoCedarEntity;

/// Represents supported data types for entity attributes when building Cedar schemas.
pub enum CedarType {
    /// Boolean primitive type (`true` / `false`).
    Boolean,
    /// String primitive type.
    String,
    /// Reference to another Cedar entity type.
    EntityRef(&'static str),
}

/// Builder for constructing Cedar schemas programmatically from application entities.
///
/// Registers entity types (and their attributes) from [`IntoCedarEntity`] impls
/// and emits the schema in the human-readable Cedar schema format, compatible
/// with [`cedar_policy::Schema`].
pub struct CedarSchemaBuilder {
    entity_types: HashMap<String, HashMap<String, CedarType>>,
    actions: Vec<String>,
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
        Self {
            entity_types,
            actions: Vec::new(),
        }
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

    /// Adds a custom action (e.g. `"Deactivate"`) to the schema.
    ///
    /// Applies to any `User` principal and any resource type, matching the
    /// shape of the built-in CRUD actions (`Read`/`Create`/`Update`/`Delete`).
    /// Names already covered by the built-ins or added before are ignored, so
    /// the emitted schema never declares an action twice (duplicate `action`
    /// declarations make [`cedar_policy::Schema`] parsing fail at startup).
    pub fn add_action(mut self, name: &str) -> Self {
        let builtin = matches!(name, "Read" | "Create" | "Update" | "Delete");
        if !builtin && !self.actions.iter().any(|a| a == name) {
            self.actions.push(name.to_string());
        }
        self
    }

    /// Builds the schema in the human-readable Cedar schema format.
    ///
    /// Returns a string suitable for parsing into [`cedar_policy::Schema`].
    /// Entity types come from the registered structs; actions are the CRUD
    /// four plus any added via [`add_action`](Self::add_action), all applying
    /// to any `User` principal and any resource type (omitting `resource`
    /// leaves it unconstrained in Cedar).
    pub fn build(&self) -> String {
        let mut out = String::new();
        let mut entity_names: Vec<&String> = self.entity_types.keys().collect();
        entity_names.sort();
        for name in &entity_names {
            let attrs = &self.entity_types[name.as_str()];
            out.push_str(&format!("entity {name} {{\n"));
            let mut attr_names: Vec<&String> = attrs.keys().collect();
            attr_names.sort();
            for attr in attr_names {
                let ty = match attrs[attr] {
                    // `Bool`, not `Boolean` — cedar-policy 4.x schema DSL resolves `Boolean`
                    // as an entity type (the one written `CedarType::Boolean` semantically is a
                    // cedar `boolean`, the primitive spelled `Bool`), so an attribute emitted
                    // as `Boolean` makes the whole schema unparseable at startup.
                    CedarType::Boolean => "Bool".to_owned(),
                    CedarType::String => "String".to_owned(),
                    CedarType::EntityRef(target) => target.to_owned(),
                };
                out.push_str(&format!("  {attr}: {ty},\n"));
            }
            out.push_str("};\n\n");
        }
        for action in ["Read", "Create", "Update", "Delete"]
            .into_iter()
            .chain(self.actions.iter().map(String::as_str))
        {
            // `User` is the principal type; every other registered entity type
            // is a resource. `appliesTo` requires both declarations in cedar 4.x.
            let resources: Vec<&str> = entity_names
                .iter()
                .filter(|n| n.as_str() != "User")
                .map(|n| n.as_str())
                .collect();
            if resources.is_empty() {
                out.push_str(&format!("action {action};\n\n"));
            } else {
                out.push_str(&format!(
                    "action {action} appliesTo {{\n  principal: User,\n  resource: [{}]\n}};\n\n",
                    resources.join(", ")
                ));
            }
        }
        out
    }
}
