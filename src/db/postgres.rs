use async_trait::async_trait;

use crate::{
    db::LinksDB,
    db_pool::DbPool,
    models::{CreateLink, FetchLink},
    schema,
};

#[derive(Clone)]
pub struct PostgresDb {
    db: DbPool,
}

impl PostgresDb {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }
}

#[async_trait]
impl LinksDB for PostgresDb {
    async fn create(&self, link: &CreateLink) -> Result<CreateLink, super::DbError> {
        use diesel_async::RunQueryDsl;

        let affected = diesel::insert_into(schema::links::table)
            .values(link)
            .execute(&mut self.db.0.get().await?)
            .await?;

        if affected != 1 {
            return Err(super::DbError::General("Failed to create link".to_string()));
        }

        Ok(link.clone())
    }

    async fn get(&self, id: &str) -> Result<Option<FetchLink>, super::DbError> {
        use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
        use diesel_async::RunQueryDsl;

        Ok(schema::links::table
            .filter(schema::links::id.eq(id))
            .select(FetchLink::as_select())
            .first(&mut self.db.0.get().await?)
            .await
            .optional()?)
    }
}
