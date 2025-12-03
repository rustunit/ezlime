use diesel::prelude::*;

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::links)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreateLink {
    pub id: String,
    pub url: String,
    pub key: String,
}

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::x402)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreateTransaction {
    pub network: String,
    pub tx_hash: String,
    pub link_id: String,
}

#[derive(Queryable, Selectable, Clone, PartialEq, Eq, Debug)]
#[diesel(table_name = crate::schema::links)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FetchLink {
    pub id: String,
    pub url: String,
}
