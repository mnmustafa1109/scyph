//! AWS Cedar Policy Language engine integration for dynamic policy evaluation.
//!
//! Provides [`CedarAuthorizer`] to evaluate authorization rules written in Cedar policy language (`.cedar`),
//! [`IntoCedarEntity`] for converting database models into Cedar entities, [`CedarEntityBuilder`],
//! and [`CedarSchemaBuilder`] for programmatic Cedar JSON schema construction.
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

pub mod authorizer;
pub mod entity;
pub mod error;
pub mod schema;

pub use authorizer::CedarAuthorizer;
pub use entity::{CedarEntityBuilder, IntoCedarEntity};
pub use error::CedarError;
pub use schema::{CedarSchemaBuilder, CedarType};
