//! Dynamic SQL query composition extensions for `sqlx::QueryBuilder`.
//!
//! Provides type-safe traits to append full-text search ILIKE filters, column equality constraints,
//! ORDER BY sorts, LIMIT / OFFSET pagination, and composite request parameters to PostgreSQL query builders.

pub mod constants;
pub mod filter;
pub mod pagination;
pub mod params;
pub mod search;
pub mod sort;

pub use constants::*;
pub use filter::*;
pub use pagination::*;
pub use params::*;
pub use search::*;
pub use sort::*;
