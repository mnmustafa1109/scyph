//! Integration tests for scyph-db QueryBuilder extensions.

use scyph_db::query::{
    ApplyFiltering, ApplyPagination, ApplyRequestParams, ApplySearch, ApplySorting,
};
use scyph_extractors::query::{
    FilterParams, PaginationParams, RequestParams, SearchParams, SortOrder, SortParams,
};
use sqlx::{Postgres, QueryBuilder};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumIter)]
enum UserSearchField {
    #[default]
    Name,
    Email,
}

impl From<UserSearchField> for &'static str {
    fn from(field: UserSearchField) -> Self {
        match field {
            UserSearchField::Name => "users.name",
            UserSearchField::Email => "users.email",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum UserSortField {
    #[default]
    CreatedAt,
    Name,
}

impl From<UserSortField> for &'static str {
    fn from(field: UserSortField) -> Self {
        match field {
            UserSortField::CreatedAt => "users.created_at",
            UserSortField::Name => "users.name",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum UserFilterField {
    #[default]
    Status,
}

impl From<UserFilterField> for &'static str {
    fn from(field: UserFilterField) -> Self {
        match field {
            UserFilterField::Status => "users.status",
        }
    }
}

#[test]
fn test_apply_pagination() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let pagination = PaginationParams { page: 2, limit: 15 };
    qb.apply_pagination(&pagination);
    let sql = qb.sql().as_str().to_string();
    assert!(sql.contains("LIMIT"));
    assert!(sql.contains("OFFSET"));
}

#[test]
fn test_apply_search() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let search = SearchParams {
        q: Some("alice".to_string()),
    };
    qb.apply_search::<UserSearchField>(&search);
    let sql = qb.sql().as_str().to_string();
    assert!(sql.contains("users.name ILIKE"));
    assert!(sql.contains("users.email ILIKE"));
    assert!(sql.contains("OR"));
}

#[test]
fn test_apply_search_empty_whitespace() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let search = SearchParams {
        q: Some("   ".to_string()),
    };
    qb.apply_search::<UserSearchField>(&search);
    let sql = qb.sql().as_str().to_string();
    assert!(!sql.contains("ILIKE"));
}

#[test]
fn test_apply_sorting_asc() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let sort = SortParams {
        sort_by: Some(UserSortField::Name),
        sort_order: Some(SortOrder::Asc),
    };
    qb.apply_sorting(&sort);
    let sql = qb.sql().as_str().to_string();
    assert!(sql.contains("ORDER BY users.name ASC"));
}

#[test]
fn test_apply_sorting_desc() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let sort = SortParams {
        sort_by: Some(UserSortField::CreatedAt),
        sort_order: Some(SortOrder::Desc),
    };
    qb.apply_sorting(&sort);
    let sql = qb.sql().as_str().to_string();
    assert!(sql.contains("ORDER BY users.created_at DESC"));
}

#[test]
fn test_apply_filtering() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let filter = FilterParams {
        filter_by: Some(UserFilterField::Status),
        filter_value: Some("active".to_string()),
    };
    qb.apply_filtering(&filter);
    let sql = qb.sql().as_str().to_string();
    assert!(sql.contains("users.status::text ="));
}

#[test]
fn test_apply_request_params_composite() {
    let mut qb = QueryBuilder::<Postgres>::new("SELECT * FROM users WHERE 1=1");
    let params = RequestParams {
        pagination: PaginationParams { page: 1, limit: 10 },
        sort: SortParams {
            sort_by: Some(UserSortField::CreatedAt),
            sort_order: Some(SortOrder::Desc),
        },
        filter: FilterParams {
            filter_by: Some(UserFilterField::Status),
            filter_value: Some("active".to_string()),
        },
        search: SearchParams {
            q: Some("bob".to_string()),
        },
    };

    qb.apply_request_params::<UserSearchField>(&params);
    let sql = qb.sql().as_str().to_string();
    assert!(sql.contains("ILIKE"));
    assert!(sql.contains("users.status::text ="));
    assert!(sql.contains("ORDER BY users.created_at DESC"));
    assert!(sql.contains("LIMIT"));
    assert!(sql.contains("OFFSET"));
}
