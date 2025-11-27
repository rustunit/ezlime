use crate::models::{CreateLink, FetchLink};
use async_trait::async_trait;
use diesel::result::DatabaseErrorKind;
use thiserror::Error;

mod postgres;

pub use postgres::PostgresDb;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    General(String),
    #[error("Duplicate Id Error")]
    DuplicateId,
}

impl From<diesel::result::Error> for DbError {
    fn from(e: diesel::result::Error) -> Self {
        match e {
            diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                DbError::DuplicateId
            }
            _ => DbError::General(e.to_string()),
        }
    }
}

impl From<deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>> for DbError {
    fn from(e: deadpool::managed::PoolError<diesel_async::pooled_connection::PoolError>) -> Self {
        DbError::General(e.to_string())
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait LinksDB: Send + Sync {
    async fn create(&self, link: &CreateLink) -> Result<CreateLink, DbError>;
    async fn get(&self, id: &str) -> Result<Option<FetchLink>, DbError>;
}
