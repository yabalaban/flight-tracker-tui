use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Flight {
    pub flight_number: String,
    pub callsign: String,
    pub icao24: String,

    pub status: FlightStatus,

    // Position data (from OpenSky)
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude_ft: Option<f64>,
    pub heading: Option<f64>,
    pub vertical_rate: Option<f64>,
    pub ground_speed_kts: Option<f64>,
    pub on_ground: bool,
    pub squawk: Option<String>,

    // Route data (from AviationStack)
    pub airline: Option<String>,
    pub aircraft_type: Option<String>,
    pub registration: Option<String>,

    pub origin: Option<Airport>,
    pub destination: Option<Airport>,

    pub departure_scheduled: Option<String>,
    pub departure_estimated: Option<String>,
    pub departure_actual: Option<String>,
    pub departure_delay: Option<i32>,

    pub arrival_scheduled: Option<String>,
    pub arrival_estimated: Option<String>,
    pub arrival_actual: Option<String>,
    pub arrival_delay: Option<i32>,

    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct Airport {
    pub name: Option<String>,
    pub iata: Option<String>,
    pub icao: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum FlightStatus {
    #[default]
    Unknown,
    Scheduled,
    EnRoute,
    Landed,
    OnGround,
    Delayed,
    Cancelled,
    NotFound,
}

impl FlightStatus {
    pub fn from_api_status(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "scheduled" => FlightStatus::Scheduled,
            "active" | "en-route" => FlightStatus::EnRoute,
            "landed" => FlightStatus::Landed,
            "delayed" => FlightStatus::Delayed,
            "cancelled" => FlightStatus::Cancelled,
            _ => FlightStatus::Unknown,
        }
    }
}

impl std::fmt::Display for FlightStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlightStatus::Unknown => write!(f, "Unknown"),
            FlightStatus::Scheduled => write!(f, "Scheduled"),
            FlightStatus::EnRoute => write!(f, "En Route"),
            FlightStatus::Landed => write!(f, "Landed"),
            FlightStatus::OnGround => write!(f, "On Ground"),
            FlightStatus::Delayed => write!(f, "Delayed"),
            FlightStatus::Cancelled => write!(f, "Cancelled"),
            FlightStatus::NotFound => write!(f, "Not Found"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flight_status_from_api_status() {
        assert_eq!(FlightStatus::from_api_status("scheduled"), FlightStatus::Scheduled);
        assert_eq!(FlightStatus::from_api_status("SCHEDULED"), FlightStatus::Scheduled);
        assert_eq!(FlightStatus::from_api_status("active"), FlightStatus::EnRoute);
        assert_eq!(FlightStatus::from_api_status("en-route"), FlightStatus::EnRoute);
        assert_eq!(FlightStatus::from_api_status("landed"), FlightStatus::Landed);
        assert_eq!(FlightStatus::from_api_status("delayed"), FlightStatus::Delayed);
        assert_eq!(FlightStatus::from_api_status("cancelled"), FlightStatus::Cancelled);
        assert_eq!(FlightStatus::from_api_status("unknown_status"), FlightStatus::Unknown);
        assert_eq!(FlightStatus::from_api_status(""), FlightStatus::Unknown);
    }

    #[test]
    fn test_flight_status_display() {
        assert_eq!(format!("{}", FlightStatus::Unknown), "Unknown");
        assert_eq!(format!("{}", FlightStatus::Scheduled), "Scheduled");
        assert_eq!(format!("{}", FlightStatus::EnRoute), "En Route");
        assert_eq!(format!("{}", FlightStatus::Landed), "Landed");
        assert_eq!(format!("{}", FlightStatus::OnGround), "On Ground");
        assert_eq!(format!("{}", FlightStatus::Delayed), "Delayed");
        assert_eq!(format!("{}", FlightStatus::Cancelled), "Cancelled");
        assert_eq!(format!("{}", FlightStatus::NotFound), "Not Found");
    }

    #[test]
    fn test_flight_default() {
        let flight = Flight::default();

        assert_eq!(flight.flight_number, "");
        assert_eq!(flight.status, FlightStatus::Unknown);
        assert!(!flight.on_ground);
        assert!(flight.latitude.is_none());
        assert!(flight.origin.is_none());
    }

    #[test]
    fn test_airport_default() {
        let airport = Airport::default();

        assert!(airport.name.is_none());
        assert!(airport.iata.is_none());
        assert!(airport.icao.is_none());
    }

    #[test]
    fn test_flight_with_data() {
        let flight = Flight {
            flight_number: "UA123".to_string(),
            callsign: "UAL123".to_string(),
            status: FlightStatus::EnRoute,
            latitude: Some(37.7749),
            longitude: Some(-122.4194),
            altitude_ft: Some(35000.0),
            origin: Some(Airport {
                name: Some("San Francisco International".to_string()),
                iata: Some("SFO".to_string()),
                icao: Some("KSFO".to_string()),
            }),
            destination: Some(Airport {
                name: Some("John F Kennedy International".to_string()),
                iata: Some("JFK".to_string()),
                icao: Some("KJFK".to_string()),
            }),
            ..Default::default()
        };

        assert_eq!(flight.flight_number, "UA123");
        assert_eq!(flight.status, FlightStatus::EnRoute);
        assert_eq!(flight.latitude, Some(37.7749));
        assert!(flight.origin.is_some());
        assert_eq!(flight.origin.as_ref().unwrap().iata, Some("SFO".to_string()));
    }
}
