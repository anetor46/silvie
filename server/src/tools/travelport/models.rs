//! Travelport Stays v11 — domain + wire types.
//!
//! Two layers:
//!
//! 1. **Domain types** (public, serialised back to the LLM). Clean Rust
//!    shapes the rest of the codebase consumes — `HotelOffer`, `RateQuote`,
//!    `CancellationPolicy`, `HotelDetails`.
//! 2. **Wire types** (crate-private). Match Travelport's JSON exactly. All
//!    request/response shapes verified against the public webhelp API
//!    references (Hotel v11, `support.travelport.com/webhelp/JSONAPIs/Hotelv11`).
//!
//! Travelport identifies a property by the pair (chainCode, propertyCode),
//! e.g. `("DT", "35429")`. In our domain `property_id` is the concatenated
//! `chainCode-propertyCode` string (`"DT-35429"`) so the LLM only carries
//! one identifier; we split on the dash when calling Travelport.

use serde::{Deserialize, Serialize};

// ── Domain (LLM-facing) ─────────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone)]
pub struct HotelOffer {
    /// `chainCode-propertyCode` — pass verbatim to `hotel_details` and
    /// `hotel_availability`. Split internally before talking to Travelport.
    pub property_id: String,
    pub name: String,
    pub address: String,
    pub city: String,
    pub stars: Option<f32>,
    pub image_url: Option<String>,
    pub lowest_total_minor_units: Option<i64>,
    pub lowest_per_night_minor_units: Option<i64>,
    pub currency: String,
    /// Distance in km from the searched location, if Travelport supplies it.
    pub distance_km: Option<f32>,
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
}

#[derive(Serialize, Debug, Clone)]
pub struct RateQuote {
    /// CatalogOffering identifier — pass to `hotel_book` so the
    /// reference-payload booking can reuse the cached offer.
    pub offer_id: String,
    /// Travelport's `bookingCode` — the human-ish rate token.
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
    pub refund_deadline: Option<String>,
    pub penalty_minor_units: Option<i64>,
    pub description: Option<String>,
}

// ── Identifier helpers ──────────────────────────────────────────────────────

/// Split our composite `property_id` ("DT-35429") into Travelport's
/// `(chainCode, propertyCode)` pair.
pub(super) fn split_property_id(id: &str) -> Option<(String, String)> {
    let (chain, code) = id.split_once('-')?;
    if chain.is_empty() || code.is_empty() {
        return None;
    }
    Some((chain.to_string(), code.to_string()))
}

pub(super) fn join_property_id(chain: &str, code: &str) -> String {
    format!("{chain}-{code}")
}

/// Convert major-unit amount to minor units (cents). Travelport only quotes
/// USD/EUR/GBP/AUD/etc. — all decimal currencies — so ×100 is always right.
pub(super) fn to_minor_units(amount: Option<f64>) -> Option<i64> {
    let v = amount?;
    if !v.is_finite() {
        return None;
    }
    Some((v * 100.0).round() as i64)
}

// ── Search by Location ──────────────────────────────────────────────────────
// POST /11/hotel/search/properties/search

#[derive(Serialize, Debug)]
pub(super) struct SearchByLocationReq {
    #[serde(rename = "PropertiesQuerySearch")]
    pub query: PropertiesQuerySearch,
}

#[derive(Serialize, Debug)]
pub(super) struct PropertiesQuerySearch {
    #[serde(rename = "CheckInDate")]
    pub check_in_date: String,
    #[serde(rename = "CheckOutDate")]
    pub check_out_date: String,
    #[serde(rename = "AggregatorList")]
    pub aggregator_list: Vec<&'static str>,
    #[serde(rename = "RoomStayCandidate")]
    pub room_stay_candidate: Vec<RoomStayCandidate>,
    #[serde(rename = "SearchBy")]
    pub search_by: SearchByCity,
}

#[derive(Serialize, Debug)]
pub(super) struct RoomStayCandidate {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "GuestCounts")]
    pub guest_counts: GuestCounts,
}

#[derive(Serialize, Debug)]
pub(super) struct GuestCounts {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "GuestCount")]
    pub guest_count: Vec<GuestCount>,
}

#[derive(Serialize, Debug)]
pub(super) struct GuestCount {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    pub count: u32,
    #[serde(rename = "ageQualifyingCode")]
    pub age_qualifying_code: &'static str,
}

#[derive(Serialize, Debug)]
pub(super) struct SearchByCity {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "SearchRadius")]
    pub search_radius: SearchRadius,
    #[serde(rename = "SearchCity")]
    pub search_city: String,
}

#[derive(Serialize, Debug)]
pub(super) struct SearchRadius {
    pub value: u32,
    #[serde(rename = "unitOfDistance")]
    pub unit_of_distance: &'static str,
}

#[allow(dead_code)] // Properties.numberOfPages / Identifier fields are present in the
                    // response but not yet consumed; serde populates them during parse.
#[derive(Deserialize, Debug)]
pub(super) struct SearchResp {
    #[serde(rename = "PropertiesResponse")]
    pub properties_response: Option<PropertiesResponse>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct PropertiesResponse {
    #[serde(rename = "Properties")]
    pub properties: Option<Properties>,
    #[serde(default, rename = "traceId")]
    pub trace_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct Properties {
    #[serde(default, rename = "totalProperties")]
    pub total_properties: Option<u32>,
    #[serde(default, rename = "PropertyInfo")]
    pub property_info: Vec<PropertyInfo>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct PropertyInfo {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "availability")]
    pub availability: Option<String>,
    #[serde(default, rename = "Distance")]
    pub distance: Option<Distance>,
    #[serde(rename = "Property")]
    pub property: Option<PropertyDetail>,
    #[serde(default, rename = "LowestAvailableRate")]
    pub lowest_available_rate: Option<MoneyValue>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct PropertyDetail {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "PropertyKey")]
    pub property_key: Option<PropertyKey>,
    #[serde(default, rename = "Rating")]
    pub rating: Vec<Rating>,
    #[serde(default, rename = "Image")]
    pub image: Vec<Image>,
    #[serde(default, rename = "Address")]
    pub address: Option<Address>,
    #[serde(default, rename = "PropertyAmenity")]
    pub property_amenity: Vec<PropertyAmenity>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(super) struct PropertyKey {
    #[serde(rename = "chainCode")]
    pub chain_code: String,
    #[serde(rename = "propertyCode")]
    pub property_code: String,
}

#[derive(Deserialize, Debug)]
pub(super) struct Rating {
    pub value: Option<f32>,
    #[serde(default)]
    #[allow(dead_code)]
    pub provider: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct Image {
    pub value: Option<String>,
    #[serde(default, rename = "dimensionCategory")]
    pub dimension_category: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct Address {
    #[serde(default, rename = "AddressLine")]
    pub address_line: Vec<String>,
    #[serde(default, rename = "City")]
    pub city: Option<String>,
    #[serde(default, rename = "StateProv")]
    pub state_prov: Option<TextValue>,
    #[serde(default, rename = "Country")]
    pub country: Option<TextValue>,
    #[serde(default, rename = "PostalCode")]
    pub postal_code: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct TextValue {
    pub value: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct PropertyAmenity {
    pub description: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub code: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct Distance {
    pub value: Option<f32>,
    #[serde(rename = "unitOfDistance")]
    pub unit_of_distance: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct MoneyValue {
    pub value: Option<f64>,
    pub code: Option<String>,
}

// ── Details ─────────────────────────────────────────────────────────────────
// GET /11/hotel/search/propertiesdetail?chainCode={XY}&propertyCode={12345}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct DetailsResp {
    #[serde(rename = "PropertiesResponse")]
    pub properties_response: Option<PropertiesResponse>,
}

// ── Availability ────────────────────────────────────────────────────────────
// POST /11/hotel/availability/catalogofferingshospitality

#[derive(Serialize, Debug)]
pub(super) struct AvailabilityReq {
    #[serde(rename = "CatalogOfferingsQueryRequest")]
    pub query: CatalogOfferingsQueryRequest,
}

#[derive(Serialize, Debug)]
pub(super) struct CatalogOfferingsQueryRequest {
    #[serde(rename = "CatalogOfferingsRequest")]
    pub catalog_offerings_request: Vec<CatalogOfferingsRequestHospitality>,
}

#[derive(Serialize, Debug)]
pub(super) struct CatalogOfferingsRequestHospitality {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "requestedCurrency")]
    pub requested_currency: String,
    #[serde(rename = "StayDates")]
    pub stay_dates: StayDates,
    #[serde(rename = "HotelSearchCriterion")]
    pub hotel_search_criterion: HotelSearchCriterion,
}

#[derive(Serialize, Debug)]
pub(super) struct StayDates {
    pub start: String,
    pub end: String,
}

#[derive(Serialize, Debug)]
pub(super) struct HotelSearchCriterion {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "numberOfRooms")]
    pub number_of_rooms: u32,
    #[serde(rename = "AggregatorList")]
    pub aggregator_list: Vec<&'static str>,
    #[serde(rename = "PropertyRequest")]
    pub property_request: Vec<PropertyRequest>,
    #[serde(rename = "RoomStayCandidates")]
    pub room_stay_candidates: RoomStayCandidates,
}

#[derive(Serialize, Debug)]
pub(super) struct PropertyRequest {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "PropertyKey")]
    pub property_key: PropertyKey,
}

#[derive(Serialize, Debug)]
pub(super) struct RoomStayCandidates {
    #[serde(rename = "RoomStayCandidate")]
    pub room_stay_candidate: Vec<AvailRoomStayCandidate>,
}

#[derive(Serialize, Debug)]
pub(super) struct AvailRoomStayCandidate {
    #[serde(rename = "GuestCounts")]
    pub guest_counts: AvailGuestCounts,
}

#[derive(Serialize, Debug)]
pub(super) struct AvailGuestCounts {
    #[serde(rename = "GuestCount")]
    pub guest_count: Vec<AvailGuestCount>,
}

#[derive(Serialize, Debug)]
pub(super) struct AvailGuestCount {
    pub count: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct AvailabilityResp {
    #[serde(rename = "CatalogOfferingsResponse")]
    pub catalog_offerings_response: Option<CatalogOfferingsResponse>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct CatalogOfferingsResponse {
    #[serde(default, rename = "CatalogOfferings")]
    pub catalog_offerings: Option<CatalogOfferings>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct CatalogOfferings {
    #[serde(default, rename = "CatalogOffering")]
    pub catalog_offering: Vec<CatalogOffering>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct CatalogOffering {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "Identifier")]
    pub identifier: Option<Identifier>,
    #[serde(default, rename = "Product")]
    pub product: Vec<ProductHospitality>,
    #[serde(default, rename = "TotalPrice")]
    pub total_price: Option<MoneyValue>,
    #[serde(default, rename = "Price")]
    pub price: Option<PriceDetail>,
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct Identifier {
    pub value: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub authority: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct ProductHospitality {
    #[serde(default, rename = "bookingCode")]
    pub booking_code: Option<String>,
    #[serde(default, rename = "propertyName")]
    pub property_name: Option<String>,
    #[serde(default, rename = "PropertyKey")]
    pub property_key: Option<PropertyKey>,
    #[serde(default, rename = "ProductDescription")]
    pub product_description: Option<TextValue>,
    #[serde(default, rename = "guests")]
    pub guests: Option<u32>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct PriceDetail {
    #[serde(default, rename = "TotalPrice")]
    pub total_price: Option<f64>,
    #[serde(default, rename = "CurrencyCode")]
    pub currency_code: Option<TextValue>,
    #[serde(default, rename = "Base")]
    pub base: Option<f64>,
    #[serde(default, rename = "TotalTaxes")]
    pub total_taxes: Option<f64>,
}

// ── Book (reference-payload) ────────────────────────────────────────────────
// POST /11/hotel/book/reservations/build

#[derive(Serialize, Debug)]
pub(super) struct BookReq {
    #[serde(rename = "ReservationQueryBuild")]
    pub query: ReservationQueryBuild,
}

#[derive(Serialize, Debug)]
pub(super) struct ReservationQueryBuild {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "ReservationBuild")]
    pub reservation_build: ReservationBuild,
}

#[derive(Serialize, Debug)]
pub(super) struct ReservationBuild {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "BuildFromCatalogOfferingHospitality")]
    pub build_from: BuildFromCatalogOfferingHospitality,
    #[serde(rename = "Traveler")]
    pub traveler: Vec<Traveler>,
    #[serde(rename = "FormOfPayment")]
    pub form_of_payment: Vec<FormOfPaymentPaymentCard>,
    #[serde(rename = "Payment")]
    pub payment: Vec<Payment>,
}

#[derive(Serialize, Debug)]
pub(super) struct BuildFromCatalogOfferingHospitality {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "CatalogOfferingIdentifier")]
    pub catalog_offering_identifier: Value,
}

#[derive(Serialize, Debug)]
pub(super) struct Value {
    pub value: String,
}

#[derive(Serialize, Debug)]
pub(super) struct Traveler {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "PersonName")]
    pub person_name: PersonName,
    #[serde(rename = "Email", skip_serializing_if = "Vec::is_empty")]
    pub email: Vec<EmailValue>,
}

#[derive(Serialize, Debug)]
pub(super) struct PersonName {
    #[serde(rename = "Given")]
    pub given: String,
    #[serde(rename = "Surname")]
    pub surname: String,
}

#[derive(Serialize, Debug)]
pub(super) struct EmailValue {
    pub value: String,
}

#[derive(Serialize, Debug)]
pub(super) struct FormOfPaymentPaymentCard {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "PaymentCard")]
    pub payment_card: PaymentCardDetail,
}

#[derive(Serialize, Debug)]
pub(super) struct PaymentCardDetail {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    /// "MMYY" — e.g. "1125" for November 2025.
    #[serde(rename = "expireDate")]
    pub expire_date: String,
    #[serde(rename = "CardType")]
    pub card_type: &'static str,
    /// Travelport card-code (Visa = "VI", Mastercard = "CA", Amex = "AX").
    #[serde(rename = "CardCode")]
    pub card_code: String,
    #[serde(rename = "CardHolderName")]
    pub card_holder_name: String,
    #[serde(rename = "CardNumber")]
    pub card_number: PlainTextField,
    #[serde(rename = "SeriesCode")]
    pub series_code: PlainTextField,
}

#[derive(Serialize, Debug)]
pub(super) struct PlainTextField {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "PlainText")]
    pub plain_text: String,
}

#[derive(Serialize, Debug)]
pub(super) struct Payment {
    #[serde(rename = "@type")]
    pub type_: &'static str,
    #[serde(rename = "Amount")]
    pub amount: MoneyAmountOut,
    #[serde(rename = "guaranteeInd")]
    pub guarantee_ind: bool,
    #[serde(rename = "depositInd")]
    pub deposit_ind: bool,
}

#[derive(Serialize, Debug)]
pub(super) struct MoneyAmountOut {
    pub code: String,
    pub value: f64,
}

// Book / Retrieve / Cancel — response

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct ReservationResp {
    #[serde(rename = "ReservationResponse")]
    pub reservation_response: Option<ReservationResponseInner>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct ReservationResponseInner {
    #[serde(default, rename = "Offer")]
    pub offer: Vec<RespOffer>,
    #[serde(default, rename = "Receipt")]
    pub receipt: Vec<Receipt>,
    #[serde(default, rename = "traceId")]
    pub trace_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct RespOffer {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, rename = "Identifier")]
    pub identifier: Option<Identifier>,
    #[serde(default, rename = "Product")]
    pub product: Vec<ProductHospitality>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct Receipt {
    #[serde(rename = "@type", default)]
    pub type_: Option<String>,
    #[serde(default, rename = "Confirmation")]
    pub confirmation: Option<Confirmation>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct Confirmation {
    #[serde(default, rename = "Locator")]
    pub locator: Option<Locator>,
    #[serde(default, rename = "OfferStatus")]
    pub offer_status: Option<OfferStatus>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct Locator {
    pub value: Option<String>,
    #[serde(default, rename = "locatorType")]
    pub locator_type: Option<String>,
    #[serde(default, rename = "sourceContext")]
    pub source_context: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub(super) struct OfferStatus {
    #[serde(default, rename = "Status")]
    pub status: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}
