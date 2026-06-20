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

diesel::table! {
    payment_methods (id) {
        id -> Uuid,
        user_id -> Uuid,
        organization_id -> Nullable<Uuid>,
        stripe_customer_id -> Text,
        stripe_payment_method_id -> Text,
        last4 -> Nullable<Text>,
        brand -> Nullable<Text>,
        exp_month -> Nullable<Int2>,
        exp_year -> Nullable<Int2>,
        label -> Nullable<Text>,
        is_default -> Bool,
        billing_address_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    issuing_card_log (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        payment_method_id -> Nullable<Uuid>,
        stripe_issuing_card_id -> Text,
        amount_minor_units -> Int8,
        currency -> Text,
        entity_type -> Nullable<Text>,
        entity_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        cancelled_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    integrations (id) {
        id -> Uuid,
        user_id -> Uuid,
        provider -> Text,
        provider_account_id -> Text,
        provider_account_email -> Nullable<Text>,
        access_token -> Text,
        refresh_token -> Nullable<Text>,
        token_expiry -> Nullable<Timestamptz>,
        scopes -> Array<Text>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(user_profiles -> users (user_id));
diesel::joinable!(addresses -> users (user_id));
diesel::joinable!(travel_documents -> users (user_id));
diesel::joinable!(payment_methods -> users (user_id));
diesel::joinable!(payment_methods -> addresses (billing_address_id));
diesel::joinable!(issuing_card_log -> users (user_id));
diesel::joinable!(issuing_card_log -> payment_methods (payment_method_id));
diesel::joinable!(integrations -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    users,
    user_profiles,
    addresses,
    travel_documents,
    payment_methods,
    issuing_card_log,
    integrations,
);
