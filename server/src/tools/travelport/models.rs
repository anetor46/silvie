//! Travelport Stays v11 — domain + wire types.
//!
//! Two layers:
//!
//! 1. **Domain types** (public, serialised back to the LLM). These are clean
//!    Rust shapes the rest of the codebase consumes — `HotelOffer`,
//!    `RateQuote`, `ReservationSummary`, `CancellationPolicy`.
//! 2. **Wire types** (crate-private). The deserialization counterparts that
//!    match Travelport's JSON. Exact field naming is verified against the
//!    developer portal sample payloads — `#[serde(alias = ...)]` is used
//!    liberally to absorb naming variation (snake / PascalCase) since the
//!    public webhelp TOC does not include schemas.
//!
//! When the real schema diverges, only the wire types + the `From<...>`
//! mappers here need to change.

use serde::{Deserialize, Serialize};

// ── Domain (LLM-facing) ─────────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone)]
pub struct HotelOffer {
    /// Travelport's pre-priced offer id (carry into Availability + Book).
    pub offer_id: String,
    /// Travelport's stable property id (used for Details).
    pub property_id: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub stars: Option<f32>,
    pub image_url: Option<String>,
    pub lowest_total_minor_units: Option<i64>,
    pub lowest_per_night_minor_units: Option<i64>,
    pub currency: String,
    pub refundable: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct HotelDetails {
    pub property_id: String,
    pub name: String,
    pub description: Option<String>,
    pub amenities: Vec<String>,
    pub photos: Vec<String>,
    pub address: String,
    pub city: String,
    pub policies: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RateQuote {
    pub offer_id: String,
    pub rate_id: String,
    pub room_description: Option<String>,
    pub total_minor_units: i64,
    pub currency: String,
    pub refundable: bool,
    pub cancellation_policy: Option<CancellationPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CancellationPolicy {
    pub refundable: bool,
    /// ISO timestamp before which the booking is fully refundable. `None`
    /// means non-refundable from the moment of booking.
    pub refund_deadline: Option<String>,
    pub penalty_minor_units: Option<i64>,
    pub description: Option<String>,
}


// ── Wire types — Search by Location ─────────────────────────────────────────

#[derive(Serialize, Debug)]
pub(super) struct SearchByLocationReq<'a> {
    pub location_code: &'a str,
    pub check_in: &'a str,
    pub check_out: &'a str,
    pub adults: u32,
    pub rooms: u32,
}

#[derive(Deserialize, Debug)]
pub(super) struct SearchResp {
    #[serde(default, alias = "Offers", alias = "offers", alias = "results")]
    pub offers: Vec<WireOffer>,
}

#[derive(Deserialize, Debug)]
pub(super) struct WireOffer {
    #[serde(alias = "OfferId", alias = "offerId", alias = "id")]
    pub offer_id: Option<String>,
    #[serde(alias = "PropertyId", alias = "propertyId", alias = "hotelCode")]
    pub property_id: Option<String>,
    #[serde(alias = "Name", alias = "name", alias = "hotelName")]
    pub name: Option<String>,
    #[serde(alias = "Address", alias = "address")]
    pub address: Option<WireAddress>,
    #[serde(alias = "StarRating", alias = "starRating", alias = "rating")]
    pub stars: Option<f32>,
    #[serde(alias = "ImageUrl", alias = "imageUrl", alias = "photo")]
    pub image_url: Option<String>,
    #[serde(alias = "TotalRate", alias = "totalRate", alias = "total")]
    pub total: Option<f64>,
    #[serde(alias = "PerNightRate", alias = "perNightRate", alias = "perNight")]
    pub per_night: Option<f64>,
    #[serde(alias = "CurrencyCode", alias = "currencyCode", alias = "currency", default)]
    pub currency: Option<String>,
    #[serde(alias = "Refundable", alias = "refundable", default)]
    pub refundable: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub(super) struct WireAddress {
    #[serde(alias = "Line", alias = "line1", alias = "street", default)]
    pub line: Option<String>,
    #[serde(alias = "City", alias = "city", default)]
    pub city: Option<String>,
}

// ── Wire types — Details ────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub(super) struct DetailsResp {
    #[serde(alias = "PropertyId", alias = "propertyId", alias = "id")]
    pub property_id: Option<String>,
    #[serde(alias = "Name", alias = "name", default)]
    pub name: Option<String>,
    #[serde(alias = "Description", alias = "description", default)]
    pub description: Option<String>,
    #[serde(alias = "Address", alias = "address", default)]
    pub address: Option<WireAddress>,
    #[serde(alias = "Amenities", alias = "amenities", default)]
    pub amenities: Vec<String>,
    #[serde(alias = "Photos", alias = "photos", default)]
    pub photos: Vec<String>,
    #[serde(alias = "Policies", alias = "policies", default)]
    pub policies: Option<String>,
}

// ── Wire types — Availability ───────────────────────────────────────────────

#[derive(Serialize, Debug)]
pub(super) struct AvailabilityReq<'a> {
    pub property_id: &'a str,
    pub check_in: &'a str,
    pub check_out: &'a str,
    pub adults: u32,
    pub rooms: u32,
}

#[derive(Deserialize, Debug)]
pub(super) struct AvailabilityResp {
    #[serde(default, alias = "Rates", alias = "rates", alias = "offers")]
    pub rates: Vec<WireRate>,
}

#[derive(Deserialize, Debug)]
pub(super) struct WireRate {
    #[serde(alias = "OfferId", alias = "offerId", default)]
    pub offer_id: Option<String>,
    #[serde(alias = "RateId", alias = "rateId", alias = "id")]
    pub rate_id: Option<String>,
    #[serde(alias = "RoomDescription", alias = "roomDescription", default)]
    pub room_description: Option<String>,
    #[serde(alias = "Total", alias = "total", alias = "totalRate")]
    pub total: Option<f64>,
    #[serde(alias = "CurrencyCode", alias = "currencyCode", alias = "currency")]
    pub currency: Option<String>,
    #[serde(alias = "Refundable", alias = "refundable", default)]
    pub refundable: Option<bool>,
    #[serde(alias = "CancellationPolicy", alias = "cancellationPolicy", default)]
    pub cancellation_policy: Option<WireCancellationPolicy>,
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct WireCancellationPolicy {
    #[serde(alias = "Refundable", alias = "refundable", default)]
    pub refundable: Option<bool>,
    #[serde(alias = "RefundDeadline", alias = "refundDeadline", default)]
    pub refund_deadline: Option<String>,
    #[serde(alias = "Penalty", alias = "penalty", default)]
    pub penalty: Option<f64>,
    #[serde(alias = "Description", alias = "description", default)]
    pub description: Option<String>,
}

// ── Wire types — Book ───────────────────────────────────────────────────────

#[derive(Serialize, Debug)]
pub(super) struct BookReq<'a> {
    pub property_id: &'a str,
    pub offer_id: &'a str,
    pub rate_id: &'a str,
    pub check_in: &'a str,
    pub check_out: &'a str,
    pub guests: u32,
    pub guest_name: &'a str,
    pub guest_email: Option<&'a str>,
    pub form_of_payment: BookFormOfPayment<'a>,
}

#[derive(Serialize, Debug)]
pub(super) struct BookFormOfPayment<'a> {
    pub card_number: &'a str,
    pub exp_month: u32,
    pub exp_year: u32,
    pub cvv: &'a str,
}

#[derive(Deserialize, Debug)]
pub(super) struct BookResp {
    #[serde(alias = "ReservationId", alias = "reservationId", alias = "confirmationNumber", alias = "bookingReference")]
    pub reservation_id: Option<String>,
    #[serde(alias = "Status", alias = "status", default)]
    pub status: Option<String>,
    #[serde(alias = "CancellationPolicy", alias = "cancellationPolicy", default)]
    pub cancellation_policy: Option<WireCancellationPolicy>,
}

// ── Wire types — Retrieve / Cancel ──────────────────────────────────────────

#[allow(dead_code)] // most fields are populated by serde but only `status` is consumed today
#[derive(Deserialize, Debug)]
pub(super) struct ReservationResp {
    #[serde(alias = "ReservationId", alias = "reservationId", alias = "confirmationNumber")]
    pub reservation_id: Option<String>,
    #[serde(alias = "Status", alias = "status", default)]
    pub status: Option<String>,
    #[serde(alias = "HotelName", alias = "hotelName", default)]
    pub hotel_name: Option<String>,
    #[serde(alias = "CheckIn", alias = "checkIn", default)]
    pub check_in: Option<String>,
    #[serde(alias = "CheckOut", alias = "checkOut", default)]
    pub check_out: Option<String>,
    #[serde(alias = "Total", alias = "total", default)]
    pub total: Option<f64>,
    #[serde(alias = "CurrencyCode", alias = "currencyCode", alias = "currency", default)]
    pub currency: Option<String>,
    #[serde(alias = "CancellationPolicy", alias = "cancellationPolicy", default)]
    pub cancellation_policy: Option<WireCancellationPolicy>,
}

#[allow(dead_code)] // `refund_amount`/`currency` come back from Travelport for the user's reference
#[derive(Deserialize, Debug)]
pub(super) struct CancelResp {
    #[serde(alias = "Status", alias = "status", default)]
    pub status: Option<String>,
    #[serde(alias = "RefundAmount", alias = "refundAmount", default)]
    pub refund_amount: Option<f64>,
    #[serde(alias = "CurrencyCode", alias = "currencyCode", alias = "currency", default)]
    pub currency: Option<String>,
}

// ── Mappers ────────────────────────────────────────────────────────────────

/// Convert a price in major units (f64) into minor units (i64) for the given
/// currency. Returns `None` if the input is missing or non-finite.
pub(super) fn to_minor_units(amount: Option<f64>, _currency: &str) -> Option<i64> {
    // We restrict currencies to USD/GBP/EUR (matching the CHECK constraint
    // on hotel_bookings.currency) so the multiplier is always 100. JPY/KRW
    // can be added with an explicit match when we widen the allowed set.
    let v = amount?;
    if !v.is_finite() {
        return None;
    }
    Some((v * 100.0).round() as i64)
}

impl WireCancellationPolicy {
    pub(super) fn into_domain(self, currency: &str) -> CancellationPolicy {
        CancellationPolicy {
            refundable: self.refundable.unwrap_or(false),
            refund_deadline: self.refund_deadline,
            penalty_minor_units: to_minor_units(self.penalty, currency),
            description: self.description,
        }
    }
}
