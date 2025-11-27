use diesel::prelude::*;

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::links)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreateLink {
    pub id: String,
    pub url: String,
    pub key: String,
}

#[derive(Queryable, Selectable, Clone, PartialEq, Eq, Debug)]
#[diesel(table_name = crate::schema::links)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FetchLink {
    pub id: String,
    pub url: String,
}
