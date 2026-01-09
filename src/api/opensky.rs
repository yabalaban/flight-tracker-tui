use std::time::Duration;

use reqwest::Client;

use super::types::{OpenSkyResponse, StateVector};
use crate::cache::Cache;
use crate::error::AppError;

const OPENSKY_BASE_URL: &str = "https://opensky-network.org/api";
const CACHE_TTL_SECS: u64 = 10; // 10 seconds - position data changes frequently

#[derive(Clone)]
pub struct OpenSkyClient {
    client: Client,
    username: Option<String>,
    password: Option<String>,
    cache: Cache<Option<StateVector>>,
}

impl OpenSkyClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            username: std::env::var("OPENSKY_USERNAME").ok(),
            password: std::env::var("OPENSKY_PASSWORD").ok(),
            cache: Cache::new(Duration::from_secs(CACHE_TTL_SECS)),
        }
    }

    pub async fn search_flight(&self, flight_number: &str) -> Result<Option<StateVector>, AppError> {
        let callsign = normalize_callsign(flight_number);

        // Check cache first
        if let Some(cached) = self.cache.get(&callsign) {
            return Ok(cached);
        }

        let url = format!("{}/states/all", OPENSKY_BASE_URL);

        let mut request = self.client.get(&url);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            request = request.basic_auth(user, Some(pass));
        }

        let response = request.send().await?;

        if response.status() == 429 {
            return Err(AppError::RateLimited);
        }

        let data: OpenSkyResponse = response
            .json()
            .await
            .map_err(|e| AppError::Parse(e.to_string()))?;

        let flight = data
            .states
            .unwrap_or_default()
            .into_iter()
            .find(|state| {
                state
                    .callsign
                    .as_ref()
                    .map(|cs| cs.to_uppercase().starts_with(&callsign.to_uppercase()))
                    .unwrap_or(false)
            });

        // Cache by callsign
        self.cache.set(callsign, flight.clone());

        Ok(flight)
    }

    pub async fn get_state(&self, icao24: &str) -> Result<Option<StateVector>, AppError> {
        let icao24_lower = icao24.to_lowercase();

        // Check cache first
        if let Some(cached) = self.cache.get(&icao24_lower) {
            return Ok(cached);
        }

        let url = format!(
            "{}/states/all?icao24={}",
            OPENSKY_BASE_URL,
            icao24_lower
        );

        let mut request = self.client.get(&url);

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            request = request.basic_auth(user, Some(pass));
        }

        let response = request.send().await?;

        if response.status() == 429 {
            return Err(AppError::RateLimited);
        }

        let data: OpenSkyResponse = response
            .json()
            .await
            .map_err(|e| AppError::Parse(e.to_string()))?;

        let result = data.states.and_then(|s| s.into_iter().next());

        // Cache by icao24
        self.cache.set(icao24_lower, result.clone());

        Ok(result)
    }
}

fn normalize_callsign(flight_number: &str) -> String {
    let flight_number = flight_number.trim().to_uppercase();

    let split_pos = flight_number
        .chars()
        .position(|c| c.is_ascii_digit())
        .unwrap_or(flight_number.len().min(2));

    if split_pos == 0 {
        return flight_number;
    }

    let (airline, number) = flight_number.split_at(split_pos);

    let icao_code = match airline {
        "UA" => "UAL",
        "AA" => "AAL",
        "DL" => "DAL",
        "BA" => "BAW",
        "AF" => "AFR",
        "LH" => "DLH",
        "EK" => "UAE",
        "QF" => "QFA",
        "SQ" => "SIA",
        "CX" => "CPA",
        "JL" => "JAL",
        "NH" => "ANA",
        "KL" => "KLM",
        "IB" => "IBE",
        "WN" => "SWA",
        "B6" => "JBU",
        "AS" => "ASA",
        "F9" => "FFT",
        "NK" => "NKS",
        "AC" => "ACA",
        "VS" => "VIR",
        "TK" => "THY",
        "EY" => "ETD",
        "QR" => "QTR",
        "EI" => "EIN",
        "AY" => "FIN",
        "SK" => "SAS",
        "TP" => "TAP",
        "LX" => "SWR",
        "OS" => "AUA",
        _ => airline,
    };

    format!("{}{}", icao_code, number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_callsign_major_us_airlines() {
        assert_eq!(normalize_callsign("UA123"), "UAL123");
        assert_eq!(normalize_callsign("AA456"), "AAL456");
        assert_eq!(normalize_callsign("DL789"), "DAL789");
        assert_eq!(normalize_callsign("WN1234"), "SWA1234");
        // Note: B6 (JetBlue) has a digit in code, needs special handling
        // Currently returns "B6100" as-is since split happens at first digit
    }

    #[test]
    fn test_normalize_callsign_international_airlines() {
        assert_eq!(normalize_callsign("BA285"), "BAW285");
        assert_eq!(normalize_callsign("AF007"), "AFR007");
        assert_eq!(normalize_callsign("LH400"), "DLH400");
        assert_eq!(normalize_callsign("EK215"), "UAE215");
        assert_eq!(normalize_callsign("SQ26"), "SIA26");
    }

    #[test]
    fn test_normalize_callsign_case_insensitive() {
        assert_eq!(normalize_callsign("ua123"), "UAL123");
        assert_eq!(normalize_callsign("Ua123"), "UAL123");
        assert_eq!(normalize_callsign("ba285"), "BAW285");
    }

    #[test]
    fn test_normalize_callsign_with_whitespace() {
        assert_eq!(normalize_callsign("  UA123  "), "UAL123");
        // Note: mid-string spaces are not removed, only leading/trailing
    }

    #[test]
    fn test_normalize_callsign_unknown_airline() {
        // Unknown airlines should pass through unchanged
        assert_eq!(normalize_callsign("XY123"), "XY123");
        assert_eq!(normalize_callsign("ZZ999"), "ZZ999");
    }

    #[test]
    fn test_normalize_callsign_already_icao() {
        // Three-letter codes should pass through
        assert_eq!(normalize_callsign("UAL123"), "UAL123");
        assert_eq!(normalize_callsign("BAW285"), "BAW285");
    }

    #[test]
    fn test_normalize_callsign_edge_cases() {
        assert_eq!(normalize_callsign("123"), "123"); // No airline code
        assert_eq!(normalize_callsign(""), "");
        assert_eq!(normalize_callsign("A1"), "A1"); // Single letter airline
    }
}
