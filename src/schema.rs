// @generated automatically by Diesel CLI.

diesel::table! {
    links (id) {
        id -> Varchar,
        url -> Text,
        created_at -> Timestamp,
        #[max_length = 32]
        key -> Nullable<Varchar>,
        click_count -> Int4,
        last_used -> Timestamptz,
    }
}
