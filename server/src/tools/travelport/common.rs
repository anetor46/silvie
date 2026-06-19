use serde::{Deserialize, Serialize};

// ── Public output types (returned to the LLM as JSON) ─────────────────────────

#[derive(Serialize)]
pub struct HotelProperty {
    /// Travelport property ID — needed for future booking calls.
    pub id: String,
    pub name: String,
    pub address: String,
    pub city: String,
    /// Star rating on a 1–5 scale. None if the property has no official rating.
    pub star_rating: Option<f32>,
    /// Lowest available rate across all room types for the requested stay.
    pub lowest_rate: Option<Rate>,
    /// True if at least one room type offers a refundable/cancellable rate.
    pub refundable_available: bool,
    /// Amenity labels, e.g. "Free WiFi", "Pool", "Free breakfast".
    pub amenities: Vec<String>,
    /// Straight-line distance from the city centre / searched location, in km.
    pub distance_km: Option<f32>,
}

#[derive(Serialize)]
pub struct Rate {
    pub amount: f64,
    pub currency: String,
    /// True when `amount` is a per-night figure; false when it is the full-stay total.
    pub per_night: bool,
    /// Total cost for the whole stay (always populated).
    pub total: f64,
}

// ── Internal API deserialization types ────────────────────────────────────────
// TODO: replace with exact shapes from Travelport+ API docs once credentials arrive.
// The structures below reflect the typical GDS JSON response pattern and will
// likely need field-name and nesting adjustments.

#[derive(Deserialize)]
pub(super) struct ApiSearchResponse {
    #[serde(rename = "HotelSearchResponse", alias = "hotelSearchResponse")]
    pub(super) response: Option<ApiHotelSearchResponse>,
    // Travelport+ may wrap the list at the top level instead
    #[serde(rename = "HotelProperty", alias = "hotelProperties", default)]
    pub(super) hotel_properties: Vec<ApiHotelProperty>,
}

#[derive(Deserialize)]
pub(super) struct ApiHotelSearchResponse {
    #[serde(rename = "HotelProperty", alias = "hotelProperties", default)]
    pub(super) hotel_properties: Vec<ApiHotelProperty>,
}

#[derive(Deserialize)]
pub(super) struct ApiHotelProperty {
    #[serde(rename = "HotelCode", alias = "hotelCode", alias = "id")]
    pub(super) id: Option<String>,
    #[serde(rename = "Name", alias = "name", alias = "hotelName")]
    pub(super) name: Option<String>,
    #[serde(rename = "Address", alias = "address")]
    pub(super) address: Option<ApiAddress>,
    #[serde(rename = "StarRating", alias = "starRating", alias = "rating")]
    pub(super) star_rating: Option<f32>,
    #[serde(rename = "RoomRate", alias = "roomRates", alias = "lowestRate", default)]
    pub(super) rates: Vec<ApiRate>,
    #[serde(rename = "Amenities", alias = "amenities", default)]
    pub(super) amenities: Vec<String>,
    #[serde(rename = "Distance", alias = "distance")]
    pub(super) distance: Option<ApiDistance>,
}

#[derive(Deserialize)]
pub(super) struct ApiAddress {
    #[serde(rename = "AddressLine", alias = "addressLine", alias = "street")]
    pub(super) line: Option<String>,
    #[serde(rename = "City", alias = "city")]
    pub(super) city: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct ApiRate {
    #[serde(rename = "Total", alias = "total", alias = "totalRate")]
    pub(super) total: Option<f64>,
    #[serde(rename = "ApproximateBaseRate", alias = "baseRate", alias = "perNight")]
    pub(super) per_night: Option<f64>,
    #[serde(rename = "CurrencyCode", alias = "currencyCode", alias = "currency")]
    pub(super) currency: Option<String>,
    #[serde(rename = "CancelPenalty", alias = "cancelPenalty", alias = "refundable")]
    pub(super) refundable: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct ApiDistance {
    #[serde(rename = "Value", alias = "value")]
    pub(super) value: Option<f32>,
    #[serde(rename = "Units", alias = "units")]
    pub(super) units: Option<String>,
}

// ── Mapping helpers ───────────────────────────────────────────────────────────

pub(super) fn map_property(api: ApiHotelProperty, nights: u32) -> Option<HotelProperty> {
    let id = api.id?;
    let name = api.name.unwrap_or_default();
    if name.is_empty() {
        return None;
    }

    let address = api
        .address
        .as_ref()
        .and_then(|a| a.line.clone())
        .unwrap_or_default();
    let city = api
        .address
        .as_ref()
        .and_then(|a| a.city.clone())
        .unwrap_or_default();

    let lowest_rate = api.rates.iter().filter_map(|r| {
        let total = r.total?;
        let per_night_amt = r.per_night.unwrap_or_else(|| {
            if nights > 0 { total / nights as f64 } else { total }
        });
        let currency = r.currency.clone().unwrap_or_else(|| "USD".to_string());
        Some(Rate {
            amount: per_night_amt,
            currency,
            per_night: true,
            total,
        })
    }).min_by(|a, b| a.total.partial_cmp(&b.total).unwrap_or(std::cmp::Ordering::Equal));

    let refundable_available = api.rates.iter().any(|r| r.refundable.unwrap_or(false));

    let distance_km = api.distance.and_then(|d| {
        let v = d.value?;
        let units = d.units.unwrap_or_default().to_lowercase();
        if units.contains("mi") {
            Some(v * 1.60934)
        } else {
            Some(v)
        }
    });

    Some(HotelProperty {
        id,
        name,
        address,
        city,
        star_rating: api.star_rating,
        lowest_rate,
        refundable_available,
        amenities: api.amenities,
        distance_km,
    })
}
