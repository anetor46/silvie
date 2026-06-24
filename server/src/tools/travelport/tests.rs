//! **Live** integration tests against the Travelport pre-prod sandbox.
//!
//! These hit the real API at `api.pp.travelport.net`. They exist to catch
//! the exact class of problems we kept tripping over manually:
//!   * Wrong endpoint paths
//!   * Wrong header names / values (access-group vs PCC)
//!   * Wrong request body shape (auth grant, request envelope)
//!   * Response shape mismatches with our parser
//!
//! Mocking the API can't catch any of those — only a real call can.
//!
//! ## Running
//!
//! These tests are marked `#[ignore]` so they're skipped by default. Run
//! them explicitly:
//!
//! ```sh
//! # All Travelport live tests
//! cargo test -p silvie-server tools::travelport::tests -- --ignored --nocapture
//!
//! # One specific test
//! cargo test -p silvie-server live_search_returns_hotels -- --ignored --nocapture
//! ```
//!
//! ## Configuration
//!
//! Credentials are read from `server/.env` (via `dotenvy`) or the process
//! environment. The required vars are:
//!
//! - `TRAVELPORT_CLIENT_ID`
//! - `TRAVELPORT_CLIENT_SECRET`
//! - `TRAVELPORT_USERNAME`
//! - `TRAVELPORT_PASSWORD`
//! - `TRAVELPORT_ACCESS_GROUP`
//!
//! If any are missing, each test logs a "skipping" line and exits cleanly —
//! so CI without secrets won't fail.
//!
//! ## Scope
//!
//! Read-only endpoints only: search, details, availability. Booking and
//! cancellation are NOT exercised here because:
//!   1. They would create real reservations in the sandbox.
//!   2. They issue real Stripe virtual cards (Stripe test mode does not
//!      gate that flow well from CI).
//!
//! Manual testing through the app is the right way to validate the
//! write tools end-to-end.

use chrono::{Duration, Utc};
use rig::tool::Tool;

use super::client::{TravelportClient, TravelportClientCreds, TravelportEnv};
use super::error::TravelportError;
use super::hotel_availability::{HotelAvailabilityArgs, HotelAvailabilityTool};
use super::hotel_details::{HotelDetailsArgs, HotelDetailsTool};
use super::hotel_search::{HotelSearchArgs, HotelSearchTool};

// ── Test fixture ──────────────────────────────────────────────────────────

/// Build a real `TravelportClient` from env vars (pre-prod). Returns
/// `None` and prints a clear "skipping" line if credentials are missing,
/// so `cargo test -- --ignored` can be run on a fresh checkout without
/// touching CI.
fn client_from_env() -> Option<TravelportClient> {
    // Best-effort load server/.env so devs don't have to remember to
    // export anything before `cargo test`. Failures here are silent — we
    // fall back to whatever the process already has set.
    let _ = dotenvy::from_path("server/.env");
    let _ = dotenvy::from_path(".env");

    let need = |name: &str| -> Option<String> {
        match std::env::var(name) {
            Ok(v) if !v.is_empty() => Some(v),
            _ => {
                eprintln!("[live travelport tests] skipping: env var {name} is not set");
                None
            }
        }
    };
    let client_id = need("TRAVELPORT_CLIENT_ID")?;
    let client_secret = need("TRAVELPORT_CLIENT_SECRET")?;
    let username = need("TRAVELPORT_USERNAME")?;
    let password = need("TRAVELPORT_PASSWORD")?;
    let access_group = need("TRAVELPORT_ACCESS_GROUP")?;

    Some(TravelportClient::new(TravelportClientCreds {
        client_id,
        client_secret,
        username,
        password,
        env: TravelportEnv::Dev,
        access_group,
    }))
}

/// Common search args: London, 30 days out, 1 night, 1 adult. Far enough
/// in the future that sandbox inventory should be plentiful.
fn search_args(max_results: u32) -> HotelSearchArgs {
    let check_in = (Utc::now() + Duration::days(30)).date_naive();
    let check_out = check_in + Duration::days(1);
    HotelSearchArgs {
        destination: "LON".into(),
        check_in: check_in.format("%Y-%m-%d").to_string(),
        check_out: check_out.format("%Y-%m-%d").to_string(),
        adults: Some(1),
        rooms: Some(1),
        max_results: Some(max_results),
        radius_miles: Some(5),
        max_rate_per_night: None,
        star_rating_min: None,
    }
}

// ── Search ─────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_search_returns_hotels() {
    let Some(client) = client_from_env() else { return };

    let tool = HotelSearchTool::new(client);
    let args = search_args(10);
    let dest = args.destination.clone();
    let out = tool.call(args).await.expect("search should succeed");

    eprintln!(
        "[live] search {dest}: got {} hotel(s), {} nights",
        out.hotels.len(),
        out.nights
    );
    assert!(
        !out.hotels.is_empty(),
        "expected at least one hotel for {dest} 30 days out — \
         empty result means search is broken OR sandbox inventory is empty"
    );
}

#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_search_property_ids_are_chain_dash_property_format() {
    let Some(client) = client_from_env() else { return };

    let tool = HotelSearchTool::new(client);
    let out = tool
        .call(search_args(5))
        .await
        .expect("search should succeed");
    assert!(!out.hotels.is_empty(), "need at least one hotel to inspect");

    for h in &out.hotels {
        assert!(
            h.property_id.contains('-'),
            "property_id {:?} should be chainCode-propertyCode",
            h.property_id
        );
        let (chain, code) = h.property_id.split_once('-').unwrap();
        assert!(!chain.is_empty(), "chainCode empty in {:?}", h.property_id);
        assert!(!code.is_empty(), "propertyCode empty in {:?}", h.property_id);
    }
}

#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_search_results_have_prices_and_currency() {
    let Some(client) = client_from_env() else { return };
    let tool = HotelSearchTool::new(client);
    let out = tool.call(search_args(5)).await.expect("search ok");
    assert!(!out.hotels.is_empty(), "need at least one hotel");

    // At least SOME of the returned hotels should have a price — sold-out
    // ones legitimately won't, but a search 30 days out should have priced
    // inventory.
    let priced = out
        .hotels
        .iter()
        .filter(|h| h.lowest_total_minor_units.is_some())
        .count();
    assert!(
        priced > 0,
        "no hotels had a LowestAvailableRate; got {} hotels, \
         expected at least one with a price",
        out.hotels.len()
    );

    for h in out.hotels.iter().filter(|h| h.lowest_total_minor_units.is_some()) {
        assert!(
            matches!(h.currency.as_str(), "USD" | "GBP" | "EUR" | "AUD" | "CAD" | "JPY"),
            "unexpected currency {:?}",
            h.currency
        );
        assert!(
            h.lowest_total_minor_units.unwrap() > 0,
            "non-positive total in {h:?}"
        );
    }
}

// ── Details ────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_details_for_searched_property() {
    let Some(client) = client_from_env() else { return };

    let search = HotelSearchTool::new(client.clone());
    let out = search.call(search_args(3)).await.expect("search ok");
    let first = out
        .hotels
        .first()
        .expect("need at least one hotel from search");

    let details = HotelDetailsTool::new(client);
    let d = details
        .call(HotelDetailsArgs {
            property_id: first.property_id.clone(),
        })
        .await
        .expect("details should succeed");

    eprintln!(
        "[live] details {}: name={:?}, {} amenity(ies), {} photo(s)",
        d.property_id,
        d.name,
        d.amenities.len(),
        d.photos.len(),
    );
    assert_eq!(d.property_id, first.property_id);
    assert!(!d.name.is_empty(), "details returned empty name");
}

// ── Availability ──────────────────────────────────────────────────────────

/// **Regression test for "missing top-level CatalogOfferingsResponse".**
///
/// This is the bug class the previous version of this test missed: it
/// returned on the FIRST success, so if hotels #2..N hit a parser bug,
/// the test was green while production was broken.
///
/// Now we hit availability for EVERY hotel in the search batch and
/// classify the outcomes:
///
/// * `Ok(rates)` — fine, any rate count (including zero) is acceptable.
///   We validate the shape of the rates that come back.
/// * `ApiError` (4xx/5xx from Travelport) — acceptable, sometimes the
///   sandbox legitimately rejects an inventory query.
/// * `UnexpectedResponse` / `Parse` — **always a failure**. These mean
///   our model didn't line up with what Travelport actually sent. The
///   server has already logged the raw body via `log_and_unexpected`;
///   the test fails loud with a pointer to that log.
///
/// At least one hotel must produce rates so we know the happy path
/// actually works.
#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_availability_for_every_searched_property_never_parse_errors() {
    let Some(client) = client_from_env() else { return };

    let search = HotelSearchTool::new(client.clone());
    let args = search_args(10);
    let check_in = args.check_in.clone();
    let check_out = args.check_out.clone();
    let out = search.call(args).await.expect("search ok");
    assert!(!out.hotels.is_empty(), "search returned no hotels");

    let availability = HotelAvailabilityTool::new(client);
    let mut shape_failures: Vec<String> = Vec::new();
    let mut any_rates = false;
    let mut attempts = 0;

    for hotel in out.hotels.iter() {
        // Cap at 10 to keep wall time bounded.
        if attempts >= 10 {
            break;
        }
        attempts += 1;

        let res = availability
            .call(HotelAvailabilityArgs {
                property_id: hotel.property_id.clone(),
                check_in: check_in.clone(),
                check_out: check_out.clone(),
                adults: Some(1),
                rooms: Some(1),
                currency: None,
            })
            .await;

        match res {
            Ok(av) => {
                eprintln!(
                    "[live] {} → {} rate(s)",
                    hotel.property_id,
                    av.rates.len()
                );
                if !av.rates.is_empty() {
                    any_rates = true;
                    let r = &av.rates[0];
                    assert!(!r.offer_id.is_empty(), "rate offer_id empty");
                    assert!(!r.rate_id.is_empty(), "rate_id (bookingCode) empty");
                    assert!(r.total_minor_units > 0, "rate total non-positive");
                    assert!(!r.currency.is_empty(), "rate currency empty");
                }
            }
            Err(TravelportError::ApiError { status, .. }) => {
                eprintln!("[live] {} → supplier {} (acceptable)", hotel.property_id, status);
            }
            Err(e @ TravelportError::UnexpectedResponse(_))
            | Err(e @ TravelportError::Parse(_)) => {
                // This is THE bug we're guarding against. Record it but
                // keep going so we collect all failing properties for
                // the panic message.
                shape_failures.push(format!("{}: {e}", hotel.property_id));
            }
            Err(e) => {
                eprintln!("[live] {} → other error: {e}", hotel.property_id);
            }
        }
    }

    if !shape_failures.is_empty() {
        panic!(
            "availability response shape did not match our parser for {} of {} hotels — \
             check server logs for the raw bodies (logged at error! level via \
             log_and_unexpected). Affected:\n  - {}",
            shape_failures.len(),
            attempts,
            shape_failures.join("\n  - ")
        );
    }
    assert!(
        any_rates,
        "tried {attempts} hotels and none returned any rates — \
         either availability is broken upstream or sandbox is empty"
    );
}

#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_availability_offer_id_and_rate_id_chain_to_booking() {
    let Some(client) = client_from_env() else { return };

    let search = HotelSearchTool::new(client.clone());
    let args = search_args(5);
    let check_in = args.check_in.clone();
    let check_out = args.check_out.clone();
    let hotels = search.call(args).await.expect("search ok").hotels;

    let availability = HotelAvailabilityTool::new(client);
    for hotel in hotels.iter().take(5) {
        let Ok(av) = availability
            .call(HotelAvailabilityArgs {
                property_id: hotel.property_id.clone(),
                check_in: check_in.clone(),
                check_out: check_out.clone(),
                adults: Some(1),
                rooms: Some(1),
                currency: None,
            })
            .await
        else {
            continue;
        };
        for r in av.rates {
            // offer_id (CatalogOffering.Identifier.value) is what the book
            // call passes as CatalogOfferingIdentifier.value — it must be a
            // non-empty opaque string. Travelport's own example shape is a
            // UUID-colon-suffix; assert at minimum it's non-empty and looks
            // like an identifier (no whitespace).
            assert!(!r.offer_id.trim().is_empty(), "offer_id blank");
            assert!(
                !r.offer_id.contains(char::is_whitespace),
                "offer_id has whitespace: {:?}",
                r.offer_id
            );
            // rate_id (bookingCode) is short alphanumeric.
            assert!(!r.rate_id.trim().is_empty(), "rate_id blank");
        }
        return;
    }
    panic!("could not get availability across top 5 hotels — see prior output");
}

// ── Negative paths ────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "hits Travelport pre-prod sandbox; run with --ignored"]
async fn live_search_for_nonsense_city_returns_empty_or_4xx() {
    let Some(client) = client_from_env() else { return };
    let tool = HotelSearchTool::new(client);
    let mut args = search_args(5);
    // ZZZ isn't a real IATA city code.
    args.destination = "ZZZ".into();

    match tool.call(args).await {
        Ok(out) => {
            assert!(
                out.hotels.is_empty(),
                "expected zero hotels for ZZZ, got {}",
                out.hotels.len()
            );
        }
        Err(e) => {
            // A 4xx is also an acceptable signal; what we want to know is
            // that we don't silently get back fake "matches".
            eprintln!("[live] ZZZ search returned an error (acceptable): {e}");
        }
    }
}
