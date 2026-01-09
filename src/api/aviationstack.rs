//! AviationStack API client for flight schedule data.
//!
//! Provides route information, departure/arrival times, and delay data.
//! Uses persistent disk cache to minimize API calls (free tier: 100/month).

use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::cache::PersistentCache;
use crate::error::AppError;

const AVIATIONSTACK_BASE_URL: &str = "http://api.aviationstack.com/v1";
const CACHE_TTL_SECS: u64 = 86400; // 24 hours - schedule data rarely changes
const CACHE_FILE: &str = "schedule_cache.json";

/// Client for the AviationStack API.
#[derive(Clone)]
pub struct AviationStackClient {
    client: Client,
    api_key: Option<String>,
    cache: PersistentCache<Option<FlightData>>,
}

#[derive(Debug, Deserialize)]
pub struct AviationStackResponse {
    pub data: Option<Vec<FlightData>>,
}

/// Flight data from AviationStack API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FlightData {
    pub flight_status: Option<String>,
    pub departure: Option<AirportInfo>,
    pub arrival: Option<AirportInfo>,
    pub airline: Option<AirlineInfo>,
    pub flight: Option<FlightInfo>,
    pub aircraft: Option<AircraftInfo>,
}

/// Airport information including schedule times.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportInfo {
    pub airport: Option<String>,
    pub iata: Option<String>,
    pub icao: Option<String>,
    pub scheduled: Option<String>,
    pub estimated: Option<String>,
    pub actual: Option<String>,
    pub delay: Option<i32>,
}

/// Airline information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AirlineInfo {
    pub name: Option<String>,
    pub iata: Option<String>,
}

/// Flight number information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FlightInfo {
    pub iata: Option<String>,
    pub icao: Option<String>,
    pub number: Option<String>,
}

/// Aircraft information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftInfo {
    pub registration: Option<String>,
    pub iata: Option<String>,
    pub icao: Option<String>,
}

impl AviationStackClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: std::env::var("AVIATIONSTACK_API_KEY").ok(),
            cache: PersistentCache::new(Duration::from_secs(CACHE_TTL_SECS), CACHE_FILE),
        }
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    pub async fn get_flight(&self, flight_number: &str) -> Result<Option<FlightData>, AppError> {
        let api_key = match &self.api_key {
            Some(key) => key,
            None => return Ok(None),
        };

        // Clean flight number (remove spaces, uppercase)
        let flight_iata = flight_number.trim().to_uppercase().replace(' ', "");

        // Check cache first
        if let Some(cached) = self.cache.get(&flight_iata) {
            return Ok(cached);
        }

        let url = format!(
            "{}/flights?access_key={}&flight_iata={}",
            AVIATIONSTACK_BASE_URL, api_key, flight_iata
        );

        let response = self.client.get(&url).send().await?;

        if response.status() == 429 {
            return Err(AppError::RateLimited);
        }

        let data: AviationStackResponse = response
            .json()
            .await
            .map_err(|e| AppError::Parse(e.to_string()))?;

        let result = data.data.and_then(|flights| flights.into_iter().next());

        // Cache the result (even if None, to avoid repeated lookups)
        self.cache.set(flight_iata, result.clone());

        Ok(result)
    }
}
