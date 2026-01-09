//! OpenSky Network API response types.
//!
//! These types represent the JSON response from the OpenSky Network REST API.
//! Some fields are deserialized but not actively used - they are kept for
//! API completeness and potential future use.

use serde::Deserialize;

/// Response from the OpenSky `/states/all` endpoint.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OpenSkyResponse {
    /// Unix timestamp of the response.
    pub time: i64,
    /// List of aircraft state vectors.
    pub states: Option<Vec<StateVector>>,
}

/// Aircraft state vector from ADS-B data.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StateVector {
    /// ICAO 24-bit transponder address (hex).
    pub icao24: String,
    /// Callsign of the aircraft.
    pub callsign: Option<String>,
    /// Country of aircraft registration.
    pub origin_country: String,
    /// Unix timestamp of last position update.
    pub time_position: Option<i64>,
    /// Unix timestamp of last contact.
    pub last_contact: i64,
    /// Longitude in decimal degrees.
    pub longitude: Option<f64>,
    /// Latitude in decimal degrees.
    pub latitude: Option<f64>,
    /// Barometric altitude in meters.
    pub baro_altitude: Option<f64>,
    /// Whether the aircraft is on the ground.
    pub on_ground: bool,
    /// Ground speed in m/s.
    pub velocity: Option<f64>,
    /// True track (heading) in degrees.
    pub true_track: Option<f64>,
    /// Vertical rate in m/s.
    pub vertical_rate: Option<f64>,
    /// Geometric (GPS) altitude in meters.
    pub geo_altitude: Option<f64>,
    /// Transponder squawk code.
    pub squawk: Option<String>,
}

impl<'de> Deserialize<'de> for StateVector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, SeqAccess, Visitor};

        struct StateVectorVisitor;

        impl<'de> Visitor<'de> for StateVectorVisitor {
            type Value = StateVector;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of 17 elements")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let icao24: String = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(0, &self))?;
                let callsign: Option<String> = seq.next_element()?.unwrap_or(None);
                let origin_country: String = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(2, &self))?;
                let time_position: Option<i64> = seq.next_element()?.unwrap_or(None);
                let last_contact: i64 = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(4, &self))?;
                let longitude: Option<f64> = seq.next_element()?.unwrap_or(None);
                let latitude: Option<f64> = seq.next_element()?.unwrap_or(None);
                let baro_altitude: Option<f64> = seq.next_element()?.unwrap_or(None);
                let on_ground: bool = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(8, &self))?;
                let velocity: Option<f64> = seq.next_element()?.unwrap_or(None);
                let true_track: Option<f64> = seq.next_element()?.unwrap_or(None);
                let vertical_rate: Option<f64> = seq.next_element()?.unwrap_or(None);
                let _sensors: Option<Vec<i32>> = seq.next_element()?.unwrap_or(None);
                let geo_altitude: Option<f64> = seq.next_element()?.unwrap_or(None);
                let squawk: Option<String> = seq.next_element()?.unwrap_or(None);
                let _spi: Option<bool> = seq.next_element()?.unwrap_or(None);
                let _position_source: Option<i32> = seq.next_element()?.unwrap_or(None);

                Ok(StateVector {
                    icao24,
                    callsign: callsign.map(|s| s.trim().to_string()),
                    origin_country,
                    time_position,
                    last_contact,
                    longitude,
                    latitude,
                    baro_altitude,
                    on_ground,
                    velocity,
                    true_track,
                    vertical_rate,
                    geo_altitude,
                    squawk,
                })
            }
        }

        deserializer.deserialize_seq(StateVectorVisitor)
    }
}
