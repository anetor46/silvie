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

diesel::table! {
    user_profiles (user_id) {
        user_id -> Uuid,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
        phone -> Nullable<Text>,
        nationality -> Nullable<Text>,
        country_of_residence -> Nullable<Text>,
        preferred_currency -> Nullable<Text>,
        preferred_language -> Nullable<Text>,
        timezone -> Nullable<Text>,
        meal_preference -> Nullable<Text>,
        seat_preference -> Nullable<Text>,
        cabin_class_preference -> Nullable<Text>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    addresses (id) {
        id -> Uuid,
        user_id -> Uuid,
        organization_id -> Nullable<Uuid>,
        #[sql_name = "type"]
        type_ -> Text,
        label -> Nullable<Text>,
        line1 -> Nullable<Text>,
        line2 -> Nullable<Text>,
        city -> Nullable<Text>,
        state -> Nullable<Text>,
        postal_code -> Nullable<Text>,
        country -> Nullable<Text>,
        is_default -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    travel_documents (id) {
        id -> Uuid,
        user_id -> Uuid,
        organization_id -> Nullable<Uuid>,
        #[sql_name = "type"]
        type_ -> Text,
        document_number -> Nullable<Text>,
        issuing_country -> Nullable<Text>,
        nationality -> Nullable<Text>,
        issue_date -> Nullable<Date>,
        expiry_date -> Nullable<Date>,
        is_primary -> Bool,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(user_profiles -> users (user_id));
diesel::joinable!(addresses -> users (user_id));
diesel::joinable!(travel_documents -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    users,
    user_profiles,
    addresses,
    travel_documents,
);
