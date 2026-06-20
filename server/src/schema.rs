// @generated automatically by Diesel CLI. Regenerate with:
//   diesel print-schema > server/src/schema.rs
// after editing migrations.

diesel::table! {
    users (id) {
        id -> Uuid,
        auth0_sub -> Text,
        email -> Text,
        name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}
