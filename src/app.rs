use std::time::Instant;

use crate::api::{FlightData, StateVector};
use crate::flight::{Airport, Flight, FlightStatus};
use crate::history::History;
use chrono::Utc;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum AppMode {
    #[default]
    Input,
    Viewing,
}

#[derive(Debug)]
pub struct App {
    pub mode: AppMode,
    pub should_quit: bool,

    pub input_buffer: String,
    pub cursor_position: usize,

    pub tracked_flights: Vec<Flight>,
    pub selected_index: Option<usize>,

    pub loading: bool,
    pub last_error: Option<String>,
    pub status_message: Option<String>,

    pub last_api_call: Option<Instant>,
    pub update_interval_secs: u64,

    /// Flight history for quick re-tracking
    pub history: History,
    /// Currently selected history index (for cycling through history)
    pub history_index: Option<usize>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            mode: AppMode::Input,
            should_quit: false,
            input_buffer: String::new(),
            cursor_position: 0,
            tracked_flights: Vec::new(),
            selected_index: None,
            loading: false,
            last_error: None,
            status_message: None,
            last_api_call: None,
            update_interval_secs: 30,
            history: History::default(),
            history_index: None,
        }
    }
}

impl App {
    /// Create a new App with history loaded from disk.
    pub fn new() -> Self {
        Self {
            history: History::load(),
            ..Default::default()
        }
    }
}

impl App {
    pub fn input_char(&mut self, c: char) {
        self.input_buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.history_index = None; // Reset history navigation on typing
    }

    pub fn input_backspace(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            self.input_buffer.remove(self.cursor_position);
        }
        self.history_index = None; // Reset history navigation on typing
    }

    pub fn submit_input(&mut self) -> Option<String> {
        if self.input_buffer.is_empty() {
            return None;
        }
        let input = self.input_buffer.clone().to_uppercase();
        self.input_buffer.clear();
        self.cursor_position = 0;
        self.history_index = None;
        Some(input)
    }

    /// Cycle to previous history entry (up arrow in input mode).
    pub fn history_previous(&mut self) {
        if self.history.is_empty() {
            return;
        }

        let entries: Vec<_> = self.history.entries().collect();
        let new_index = match self.history_index {
            None => 0,
            Some(i) => (i + 1).min(entries.len() - 1),
        };

        self.history_index = Some(new_index);
        if let Some(entry) = entries.get(new_index) {
            self.input_buffer = entry.flight_number.clone();
            self.cursor_position = self.input_buffer.len();
        }
    }

    /// Cycle to next history entry (down arrow in input mode).
    pub fn history_next(&mut self) {
        if self.history.is_empty() {
            return;
        }

        match self.history_index {
            None => {}
            Some(0) => {
                self.history_index = None;
                self.input_buffer.clear();
                self.cursor_position = 0;
            }
            Some(i) => {
                let entries: Vec<_> = self.history.entries().collect();
                self.history_index = Some(i - 1);
                if let Some(entry) = entries.get(i - 1) {
                    self.input_buffer = entry.flight_number.clone();
                    self.cursor_position = self.input_buffer.len();
                }
            }
        }
    }

    pub fn select_next(&mut self) {
        if self.tracked_flights.is_empty() {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(i) => (i + 1) % self.tracked_flights.len(),
            None => 0,
        });
    }

    pub fn select_previous(&mut self) {
        if self.tracked_flights.is_empty() {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(0) => self.tracked_flights.len() - 1,
            Some(i) => i - 1,
            None => self.tracked_flights.len() - 1,
        });
    }

    pub fn remove_selected_flight(&mut self) {
        if let Some(index) = self.selected_index {
            if index < self.tracked_flights.len() {
                self.tracked_flights.remove(index);
                if self.tracked_flights.is_empty() {
                    self.selected_index = None;
                } else if index >= self.tracked_flights.len() {
                    self.selected_index = Some(self.tracked_flights.len() - 1);
                }
            }
        }
    }

    pub fn add_flight(
        &mut self,
        flight_number: String,
        state: Option<StateVector>,
        schedule: Option<FlightData>,
    ) {
        if self
            .tracked_flights
            .iter()
            .any(|f| f.flight_number == flight_number)
        {
            self.status_message = Some(format!("Flight {} is already tracked", flight_number));
            return;
        }

        let mut flight = Flight {
            flight_number: flight_number.clone(),
            status: FlightStatus::NotFound,
            last_updated: Some(Utc::now()),
            ..Default::default()
        };

        // Apply schedule data first (from AviationStack)
        if let Some(sched) = schedule {
            apply_schedule_data(&mut flight, sched);
        }

        // Apply live position data (from OpenSky) - this may override status
        if let Some(sv) = state {
            apply_position_data(&mut flight, sv);
        }

        // Build route string for history
        let route = match (&flight.origin, &flight.destination) {
            (Some(orig), Some(dest)) => {
                let orig_code = orig.iata.as_deref().or(orig.icao.as_deref()).unwrap_or("???");
                let dest_code = dest.iata.as_deref().or(dest.icao.as_deref()).unwrap_or("???");
                Some(format!("{}→{}", orig_code, dest_code))
            }
            _ => None,
        };

        // Add to history and save
        self.history.add(flight_number, route);
        self.history.save();

        self.tracked_flights.push(flight);
        self.selected_index = Some(self.tracked_flights.len() - 1);
    }

    pub fn update_flight(&mut self, flight_number: &str, state: Option<StateVector>) {
        if let Some(flight) = self
            .tracked_flights
            .iter_mut()
            .find(|f| f.flight_number == flight_number)
        {
            if let Some(sv) = state {
                apply_position_data(flight, sv);
            }
            flight.last_updated = Some(Utc::now());
        }
    }

    pub fn should_update(&self) -> bool {
        if self.tracked_flights.is_empty() || self.loading {
            return false;
        }

        match self.last_api_call {
            Some(last) => last.elapsed().as_secs() >= self.update_interval_secs,
            None => true,
        }
    }

    pub fn seconds_until_update(&self) -> Option<u64> {
        self.last_api_call.map(|last| {
            let elapsed = last.elapsed().as_secs();
            self.update_interval_secs.saturating_sub(elapsed)
        })
    }
}

fn apply_position_data(flight: &mut Flight, sv: StateVector) {
    const METERS_TO_FEET: f64 = 3.28084;
    const MPS_TO_KNOTS: f64 = 1.94384;

    flight.callsign = sv.callsign.unwrap_or_default();
    flight.icao24 = sv.icao24;
    flight.latitude = sv.latitude;
    flight.longitude = sv.longitude;
    flight.altitude_ft = sv.baro_altitude.map(|a| a * METERS_TO_FEET);
    flight.heading = sv.true_track;
    flight.vertical_rate = sv.vertical_rate.map(|v| v * METERS_TO_FEET * 60.0);
    flight.ground_speed_kts = sv.velocity.map(|v| v * MPS_TO_KNOTS);
    flight.on_ground = sv.on_ground;
    flight.squawk = sv.squawk;

    // Update status based on live position
    if sv.on_ground {
        flight.status = FlightStatus::OnGround;
    } else {
        flight.status = FlightStatus::EnRoute;
    }
}

fn apply_schedule_data(flight: &mut Flight, data: FlightData) {
    // Status
    if let Some(status) = &data.flight_status {
        flight.status = FlightStatus::from_api_status(status);
    }

    // Airline
    if let Some(airline) = &data.airline {
        flight.airline = airline.name.clone();
    }

    // Aircraft
    if let Some(aircraft) = &data.aircraft {
        flight.aircraft_type = aircraft.iata.clone().or(aircraft.icao.clone());
        flight.registration = aircraft.registration.clone();
    }

    // Origin airport
    if let Some(dep) = &data.departure {
        flight.origin = Some(Airport {
            name: dep.airport.clone(),
            iata: dep.iata.clone(),
            icao: dep.icao.clone(),
        });
        flight.departure_scheduled = dep.scheduled.clone();
        flight.departure_estimated = dep.estimated.clone();
        flight.departure_actual = dep.actual.clone();
        flight.departure_delay = dep.delay;
    }

    // Destination airport
    if let Some(arr) = &data.arrival {
        flight.destination = Some(Airport {
            name: arr.airport.clone(),
            iata: arr.iata.clone(),
            icao: arr.icao.clone(),
        });
        flight.arrival_scheduled = arr.scheduled.clone();
        flight.arrival_estimated = arr.estimated.clone();
        flight.arrival_actual = arr.actual.clone();
        flight.arrival_delay = arr.delay;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_default() {
        let app = App::default();

        assert_eq!(app.mode, AppMode::Input);
        assert!(!app.should_quit);
        assert!(app.input_buffer.is_empty());
        assert!(app.tracked_flights.is_empty());
        assert!(app.selected_index.is_none());
        assert!(!app.loading);
    }

    #[test]
    fn test_input_char() {
        let mut app = App::default();

        app.input_char('U');
        app.input_char('A');
        app.input_char('1');

        assert_eq!(app.input_buffer, "UA1");
        assert_eq!(app.cursor_position, 3);
    }

    #[test]
    fn test_input_backspace() {
        let mut app = App::default();

        app.input_char('U');
        app.input_char('A');
        app.input_backspace();

        assert_eq!(app.input_buffer, "U");
        assert_eq!(app.cursor_position, 1);

        // Backspace on empty should do nothing
        app.input_backspace();
        app.input_backspace();
        assert_eq!(app.input_buffer, "");
        assert_eq!(app.cursor_position, 0);
    }

    #[test]
    fn test_submit_input() {
        let mut app = App::default();

        app.input_char('u');
        app.input_char('a');
        app.input_char('1');
        app.input_char('2');
        app.input_char('3');

        let result = app.submit_input();

        assert_eq!(result, Some("UA123".to_string())); // Should be uppercased
        assert!(app.input_buffer.is_empty());
        assert_eq!(app.cursor_position, 0);

        // Submit on empty should return None
        assert_eq!(app.submit_input(), None);
    }

    #[test]
    fn test_add_flight() {
        let mut app = App::default();

        app.add_flight("UA123".to_string(), None, None);

        assert_eq!(app.tracked_flights.len(), 1);
        assert_eq!(app.tracked_flights[0].flight_number, "UA123");
        assert_eq!(app.tracked_flights[0].status, FlightStatus::NotFound);
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_add_duplicate_flight() {
        let mut app = App::default();

        app.add_flight("UA123".to_string(), None, None);
        app.add_flight("UA123".to_string(), None, None);

        assert_eq!(app.tracked_flights.len(), 1);
        assert!(app.status_message.is_some());
    }

    #[test]
    fn test_select_next_previous() {
        let mut app = App::default();

        app.add_flight("UA123".to_string(), None, None);
        app.add_flight("BA285".to_string(), None, None);
        app.add_flight("AF007".to_string(), None, None);

        assert_eq!(app.selected_index, Some(2)); // Last added is selected

        app.select_previous();
        assert_eq!(app.selected_index, Some(1));

        app.select_previous();
        assert_eq!(app.selected_index, Some(0));

        app.select_previous(); // Should wrap to end
        assert_eq!(app.selected_index, Some(2));

        app.select_next(); // Should wrap to beginning
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_select_on_empty_list() {
        let mut app = App::default();

        app.select_next();
        app.select_previous();

        assert!(app.selected_index.is_none());
    }

    #[test]
    fn test_remove_selected_flight() {
        let mut app = App::default();

        app.add_flight("UA123".to_string(), None, None);
        app.add_flight("BA285".to_string(), None, None);

        app.selected_index = Some(0);
        app.remove_selected_flight();

        assert_eq!(app.tracked_flights.len(), 1);
        assert_eq!(app.tracked_flights[0].flight_number, "BA285");
        assert_eq!(app.selected_index, Some(0));
    }

    #[test]
    fn test_remove_last_flight() {
        let mut app = App::default();

        app.add_flight("UA123".to_string(), None, None);
        app.remove_selected_flight();

        assert!(app.tracked_flights.is_empty());
        assert!(app.selected_index.is_none());
    }

    #[test]
    fn test_should_update() {
        let mut app = App::default();

        // Empty list should not update
        assert!(!app.should_update());

        app.add_flight("UA123".to_string(), None, None);

        // With flights but no last call, should update
        assert!(app.should_update());

        // While loading, should not update
        app.loading = true;
        assert!(!app.should_update());
    }

    #[test]
    fn test_app_mode_default() {
        assert_eq!(AppMode::default(), AppMode::Input);
    }
}
