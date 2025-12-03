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

diesel::table! {
    x402 (network, tx_hash) {
        network -> Varchar,
        tx_hash -> Varchar,
        link_id -> Varchar,
    }
}

diesel::joinable!(x402 -> links (link_id));

diesel::allow_tables_to_appear_in_same_query!(links, x402,);
