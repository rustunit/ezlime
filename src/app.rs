use crate::{
    counter::ClickCounter,
    db::{DbError, LinksDB},
    models::{CreateLink, CreateTransaction},
};
use ezlime_rs::{CreateLinkRequest, CreatedLinkResponse};
use quick_cache::sync::Cache;
use reqwest::Url;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};
use tracing::{error, info, instrument, warn};

fn hash_string(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish() // Returns u64
}

fn link_hash(url: &str, hash_length: usize, hash_offset: u64) -> String {
    let mut hash = hash_string(url);

    if hash_offset > 0 {
        hash = hash_string(&format!("{}_{}", url, hash_offset));
    }

    let mut hash = base62::encode(hash);
    hash.truncate(hash_length);
    hash.make_ascii_lowercase();

    hash
}

#[derive(Clone)]
pub struct App {
    db: Arc<dyn LinksDB>,
    click_counter: Arc<ClickCounter>,
    prefix: String,
    hash_length: usize,
    cache: Arc<Cache<String, String>>,
}

fn validate_url(url: &str) -> Result<(), anyhow::Error> {
    let parsed = Url::parse(url)?;
    if !["http", "https"].contains(&parsed.scheme()) {
        anyhow::bail!("Only HTTP(S) URLs are allowed");
    }
    // Optional: check against blacklist of domains
    Ok(())
}

impl App {
    pub fn new(
        prefix: String,
        hash_length: usize,
        db: Arc<dyn LinksDB>,
        click_counter: Arc<ClickCounter>,
        cache_size: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            prefix,
            hash_length,
            cache: Arc::new(Cache::new(cache_size)),
            click_counter,
        })
    }

    #[instrument(skip(self), err)]
    pub async fn store_transaction(
        &self,
        link_id: String,
        tx_hash: String,
        network: String,
    ) -> Result<(), anyhow::Error> {
        let tx = CreateTransaction {
            link_id,
            tx_hash,
            network,
        };

        self.db.create_transaction(&tx).await?;

        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn create_link(
        &self,
        api_key: String,
        payload: CreateLinkRequest,
        demo_mode: bool,
    ) -> Result<CreatedLinkResponse, anyhow::Error> {
        let url = payload.url.as_str();

        validate_url(url)?;

        // If demo mode is enabled, return a demo response without creating a real link
        if demo_mode {
            info!("demo request");
            return Ok(CreatedLinkResponse::new(
                "rustunit".to_string(),
                &self.prefix,
                url.to_string(),
            ));
        }

        let mut hash_offset: u64 = 0;

        loop {
            let hash = link_hash(url, self.hash_length, hash_offset);

            info!(hash, "creating link");

            let new_link = CreateLink {
                id: hash.clone(),
                url: url.to_string(),
                key: api_key.clone(),
            };

            let res = self.db.create(&new_link).await;

            match res {
                Ok(_) => {
                    return Ok(CreatedLinkResponse::new(
                        new_link.id.clone(),
                        &self.prefix,
                        new_link.url.clone(),
                    ));
                }
                Err(DbError::DuplicateId) => {
                    info!(id = new_link.id, "id already exists");

                    if let Some(link) = self.db.get(&new_link.id).await?
                        && link.url == payload.url
                    {
                        info!(hash, "id found");

                        return Ok(CreatedLinkResponse::new(
                            new_link.id.clone(),
                            &self.prefix,
                            new_link.url.clone(),
                        ));
                    }

                    hash_offset += 1;

                    warn!(hash, hash_offset, "hash collision");
                }
                Err(e) => {
                    error!("db error: {e}");
                    anyhow::bail!("unexpected error");
                }
            };
        }
    }

    pub async fn redirect(&self, id: &str) -> Result<String, anyhow::Error> {
        if let Some(link) = self.cache.get(id) {
            self.click_counter.increment(id).await;
            info!(id, "redirect from cache");
            return Ok(link.clone());
        }

        let Some(link) = self.db.get(id).await? else {
            anyhow::bail!("unknown link")
        };

        info!(id, "redirect from db");

        self.cache.insert(id.to_string(), link.url.clone());

        self.click_counter.increment(id).await;

        Ok(link.url)
    }
}

#[cfg(test)]
mod e2e_tests {
    use super::*;
    use crate::{
        db::PostgresDb,
        db_pool::{DbPool, init_crypto_provider},
        migrations::run_migrations,
    };
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
    async fn test_app_smoke_test() {
        init_crypto_provider();

        let (_db_container, dburl) = get_postgres_testcontainer().await;

        run_migrations(&dburl).unwrap();

        let pool = DbPool::build(&dburl, 1).await.unwrap();

        let original_url = String::from("https://www.rustunit.com");
        let key = String::from("key");

        let app = App::new(
            "http://localhost".to_string(),
            6,
            Arc::new(PostgresDb::new(pool)),
            Arc::new(ClickCounter::new()),
            10,
        );
        let res = app
            .create_link(
                key.clone(),
                CreateLinkRequest {
                    url: original_url.clone(),
                },
                false,
            )
            .await
            .unwrap();

        assert_eq!(&res.id, "as9sud");
        assert_eq!(&res.original_url, &original_url);
        assert_eq!(&res.shortened_url, "http://localhost/as9sud");

        let res = app
            .create_link(
                key,
                CreateLinkRequest {
                    url: original_url.clone(),
                },
                false,
            )
            .await
            .unwrap();

        assert_eq!(&res.id, "as9sud");
        assert_eq!(&res.original_url, &original_url);
        assert_eq!(&res.shortened_url, "http://localhost/as9sud");
    }

    #[tokio::test]
    async fn test_invalid_url() {
        init_crypto_provider();

        let (_db_container, dburl) = get_postgres_testcontainer().await;

        run_migrations(&dburl).unwrap();

        let pool = DbPool::build(&dburl, 1).await.unwrap();

        let app = App::new(
            "http://localhost".to_string(),
            6,
            Arc::new(PostgresDb::new(pool)),
            Arc::new(ClickCounter::new()),
            10,
        );
        let res = app
            .create_link(
                String::from("key"),
                CreateLinkRequest {
                    url: String::from("abcde.com"),
                },
                false,
            )
            .await;

        assert!(res.is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::MockLinksDB, models::FetchLink};

    #[tokio::test]
    async fn test_caching() {
        let link = CreateLink {
            id: String::from("id"),
            url: String::from("url"),
            key: String::from("key"),
        };

        let mut db = MockLinksDB::new();
        db.expect_get().times(1).returning(move |_| {
            Ok(Some(FetchLink {
                id: link.id.clone(),
                url: link.url.clone(),
            }))
        });

        let app = App::new(
            "http://localhost".to_string(),
            6,
            Arc::new(db),
            Arc::new(ClickCounter::new()),
            10,
        );

        let res = app.redirect("foo").await.unwrap();
        assert_eq!(&res, "url");

        let res = app.redirect("foo").await.unwrap();
        assert_eq!(&res, "url");
    }
}

#[cfg(test)]
mod test_collisions {
    use crate::models::FetchLink;

    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    #[derive(Debug, Default)]
    struct MemDb {
        data: Arc<Mutex<HashMap<String, CreateLink>>>,
    }

    #[async_trait]
    impl LinksDB for MemDb {
        async fn create_transaction(&self, _tx: &CreateTransaction) -> Result<(), DbError> {
            panic!("should not be used in this test");
        }

        async fn create(&self, link: &CreateLink) -> Result<CreateLink, DbError> {
            let mut db = self.data.lock().await;

            if db.contains_key(&link.id) {
                Err(DbError::DuplicateId)
            } else {
                db.insert(link.id.clone(), link.clone());
                Ok(link.clone())
            }
        }

        async fn get(&self, id: &str) -> Result<Option<FetchLink>, DbError> {
            let db = self.data.lock().await;
            Ok(Some(FetchLink {
                id: id.to_string(),
                url: db.get(id).unwrap().url.clone(),
            }))
        }
    }

    #[tokio::test]
    async fn test_hash_collision() {
        let link1 = "https://www.google.com/search?q=foobar";
        let link2 = "https://www.google.com/search?q=foobar7";
        assert_eq!(&link_hash(link1, 1, 0)[0..1], "d");
        assert_eq!(&link_hash(link1, 1, 0)[0..1], &link_hash(link2, 1, 0)[0..1],);

        let db = MemDb::default();
        let key = String::from("key");

        let app = App::new(
            "http://localhost".to_string(),
            1,
            Arc::new(db),
            Arc::new(ClickCounter::new()),
            10,
        );

        let res1 = app
            .create_link(
                key.clone(),
                CreateLinkRequest {
                    url: link1.to_string(),
                },
                false,
            )
            .await
            .unwrap();
        let res2 = app
            .create_link(
                key,
                CreateLinkRequest {
                    url: link2.to_string(),
                },
                false,
            )
            .await
            .unwrap();

        assert_ne!(res1.id, res2.id);
    }
}
