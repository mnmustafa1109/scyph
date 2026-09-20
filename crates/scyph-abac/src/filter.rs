use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

pub struct FilterBuilder<'a>(pub(crate) QueryBuilder<'a, Postgres>);

impl<'a> FilterBuilder<'a> {
    pub fn new(init: impl Into<String>) -> Self {
        Self(QueryBuilder::new(init.into()))
    }

    pub fn push(&mut self, sql: impl Into<String>) -> &mut Self {
        self.0.push(sql);
        self
    }
}
