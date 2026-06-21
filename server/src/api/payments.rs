//! Payment HTTP handlers. The DB-backed handlers operate on `payment_methods`
//! + the linked billing address (see [`crate::repos::payments`]); the Stripe-
//! API endpoints proxy [`crate::services::stripe::PaymentClient`].

use std::sync::Arc;

use poem::{
    handler,
    http::StatusCode,
    web::{Data, Json},
};
use serde::Deserialize;

use crate::{
    auth::AuthUser,
    db::DbPool,
    error::{ApiError, ApiResult, ResultOptionExt},
    repos::payments::{
        self, CreatePaymentMethodRequest, PaymentMethodResponse, UpdateBillingRequest,
    },
    services::stripe::{PaymentClient, PaymentMethodDetails, SetupIntentResponse},
};

/// Helper: extract `&PaymentClient` from the injected `Arc<Option<…>>` or
/// produce a 503 with a clear message. Used by both Stripe-API endpoints.
fn stripe_client(arc: &Arc<Option<PaymentClient>>) -> ApiResult<&PaymentClient> {
    (**arc)
        .as_ref()
        .ok_or_else(|| ApiError::Unavailable("Stripe is not configured on the server.".into()))
}

#[derive(Deserialize)]
pub struct GetPaymentMethodRequest {
    // Kept for future validation that the PM belongs to this customer.
    #[allow(dead_code)]
    customer_id: String,
    payment_method_id: String,
}

/// POST /payment/setup — creates a Stripe Customer + SetupIntent.
#[handler]
pub async fn payment_setup_handler(
    Data(client): Data<&Arc<Option<PaymentClient>>>,
) -> ApiResult<Json<SetupIntentResponse>> {
    let client = stripe_client(client)?;
    Ok(Json(client.create_setup_intent().await?))
}

/// POST /payment/method — retrieves display-safe card metadata.
#[handler]
pub async fn payment_method_handler(
    Data(client): Data<&Arc<Option<PaymentClient>>>,
    Json(req): Json<GetPaymentMethodRequest>,
) -> ApiResult<Json<PaymentMethodDetails>> {
    let client = stripe_client(client)?;
    Ok(Json(client.get_payment_method_details(&req.payment_method_id).await?))
}

/// GET /users/me/payment-method — fetch the user's primary card + billing.
#[handler]
pub async fn get_user_payment_method_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
) -> ApiResult<Json<PaymentMethodResponse>> {
    let row = payments::fetch_payment_method(pool, auth.user.id)
        .await
        .into_required()?;
    Ok(Json(row))
}

/// POST /users/me/payment-method — find-or-update the user's primary card.
#[handler]
pub async fn create_user_payment_method_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Json(req): Json<CreatePaymentMethodRequest>,
) -> ApiResult<Json<PaymentMethodResponse>> {
    let row = payments::upsert_payment_method(pool, auth.user.id, &req).await?;
    Ok(Json(row))
}

/// DELETE /users/me/payment-method — soft-delete the user's primary card.
#[handler]
pub async fn delete_user_payment_method_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
) -> ApiResult<StatusCode> {
    let removed = payments::soft_delete_payment_method(pool, auth.user.id).await?;
    Ok(if removed {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}

/// PUT /users/me/payment-method/billing — upsert billing address + link.
#[handler]
pub async fn update_user_billing_handler(
    auth: AuthUser,
    Data(pool): Data<&DbPool>,
    Json(req): Json<UpdateBillingRequest>,
) -> ApiResult<Json<PaymentMethodResponse>> {
    let row = payments::update_billing_address(pool, auth.user.id, &req)
        .await
        .into_required()?;
    Ok(Json(row))
}
