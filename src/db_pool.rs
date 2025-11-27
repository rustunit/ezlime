use anyhow::Context;
use diesel::{ConnectionError, ConnectionResult};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::ManagerConfig;
use diesel_async::pooled_connection::deadpool::Pool;
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use rustls::ClientConfig;
use rustls_platform_verifier::ConfigVerifierExt;

#[derive(Clone)]
pub struct DbPool(
    pub  deadpool::managed::Pool<
        AsyncDieselConnectionManager<AsyncPgConnection>,
        deadpool::managed::Object<AsyncDieselConnectionManager<AsyncPgConnection>>,
    >,
);

fn establish_connection(config: &str) -> BoxFuture<'_, ConnectionResult<AsyncPgConnection>> {
    let fut = async {
        let rustls_config = ClientConfig::with_platform_verifier().unwrap();
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(rustls_config);
        let (client, conn) = tokio_postgres::connect(config, tls)
            .await
            .map_err(|e| ConnectionError::BadConnection(e.to_string()))?;

        AsyncPgConnection::try_from_client_and_connection(client, conn).await
    };
    fut.boxed()
}

impl DbPool {
    pub async fn build(db_url: &str, pool_size: usize) -> anyhow::Result<Self> {
        let mut config = ManagerConfig::default();
        config.custom_setup = Box::new(establish_connection);

        let mgr =
            AsyncDieselConnectionManager::<AsyncPgConnection>::new_with_config(db_url, config);
        let pool = Pool::builder(mgr)
            .max_size(pool_size)
            .build()
            .context("Could not build Postgres database connection pool.")?;

        {
            use diesel_async::RunQueryDsl;
            diesel::sql_query("SELECT 1")
                .execute(&mut pool.get().await?)
                .await?;
        }

        tracing::info!("db pool initialized");

        Ok(DbPool(pool))
    }
}

#[cfg(test)]
static CRYPTO_INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();

#[cfg(test)]
pub fn init_crypto_provider() {
    CRYPTO_INIT.get_or_init(|| {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .expect("CryptoProvider already installed");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::prelude::QueryableByName;
    use testcontainers::{ContainerAsync, runners::AsyncRunner};
    use testcontainers_modules::postgres::Postgres;

    async fn get_postgres_testcontainer() -> (ContainerAsync<Postgres>, String) {
        let c = Postgres::default().start().await.unwrap();

        let host_port = c.get_host_port_ipv4(5432).await.unwrap();
        let host = c.get_host().await.unwrap();

        let db_url = format!("postgres://postgres:postgres@{host}:{host_port}/postgres",);

        (c, db_url)
    }

    #[tokio::test]
    async fn test_connect_through_db_pool() {
        init_crypto_provider();

        let (_db_container, dburl) = get_postgres_testcontainer().await;

        let pool = DbPool::build(&dburl, 1).await.unwrap();
        #[derive(QueryableByName, Debug)]
        struct Recipe {
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
        }

        let recipe_list: Vec<Recipe> = {
            use diesel_async::RunQueryDsl;
            diesel::sql_query("SELECT 'Lemon Cake' as name;")
                .load(&mut pool.0.get().await.unwrap())
                .await
                .unwrap()
        };
        assert_eq!(recipe_list.first().unwrap().name, "Lemon Cake");
    }
}
