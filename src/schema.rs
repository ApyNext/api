// @generated automatically by Diesel CLI.

diesel::table! {
    account (id) {
        id -> Int8,
        #[max_length = 15]
        username -> Varchar,
        #[max_length = 1000]
        email -> Varchar,
        #[max_length = 128]
        password -> Varchar,
        birthdate -> Timestamptz,
        dark_mode -> Bool,
        #[max_length = 300]
        biography -> Varchar,
        #[max_length = 1000]
        token -> Varchar,
        is_male -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        email_verified -> Bool,
        is_banned -> Bool,
        permission -> Int4,
    }
}

diesel::table! {
    follow (id) {
        id -> Int8,
        follower_id -> Int8,
        followed_id -> Int8,
    }
}

diesel::table! {
    post (id) {
        id -> Int8,
        author_id -> Int8,
        #[max_length = 50]
        title -> Varchar,
        #[max_length = 1000]
        content -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    post_author (post_id, author_id) {
        post_id -> Int8,
        author_id -> Int8,
    }
}

diesel::joinable!(post -> account (author_id));
diesel::joinable!(post_author -> account (author_id));
diesel::joinable!(post_author -> post (post_id));

diesel::allow_tables_to_appear_in_same_query!(
    account,
    follow,
    post,
    post_author,
);
