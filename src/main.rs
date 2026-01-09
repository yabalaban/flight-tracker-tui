mod api;
mod app;
mod cache;
mod error;
mod event;
mod flight;
mod history;
mod ui;

use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use tokio::sync::mpsc;

use api::{AviationStackClient, FlightData, OpenSkyClient, StateVector};
use app::{App, AppMode};
use event::{Event, EventHandler};

enum ApiResponse {
    FlightSearch {
        flight_number: String,
        position: Result<Option<StateVector>, error::AppError>,
        schedule: Option<FlightData>,
    },
    FlightUpdate(String, Result<Option<StateVector>, error::AppError>),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    color_eyre::install()?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal).await;
    ratatui::restore();

    result
}

struct ApiClients {
    opensky: OpenSkyClient,
    aviationstack: AviationStackClient,
}

async fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    let mut events = EventHandler::new(Duration::from_millis(250));

    let clients = ApiClients {
        opensky: OpenSkyClient::new(),
        aviationstack: AviationStackClient::new(),
    };

    // Show hint if AviationStack API key is available
    if clients.aviationstack.has_api_key() {
        app.status_message = Some("AviationStack API enabled for route data".to_string());
    }

    let (api_tx, mut api_rx) = mpsc::channel::<ApiResponse>(32);

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        tokio::select! {
            Some(event) = events.next() => {
                match event {
                    Event::Key(key) => {
                        handle_key_event(&mut app, key, &clients, api_tx.clone()).await;
                    }
                    Event::Tick => {
                        handle_tick(&mut app, &clients, api_tx.clone()).await;
                    }
                    Event::Resize(_, _) => {}
                }
            }
            Some(response) = api_rx.recv() => {
                handle_api_response(&mut app, response);
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

async fn handle_key_event(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    clients: &ApiClients,
    api_tx: mpsc::Sender<ApiResponse>,
) {
    // Clear transient messages
    app.status_message = None;

    match app.mode {
        AppMode::Input => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.should_quit = true;
            } else {
                match key.code {
                    KeyCode::Enter => {
                        if let Some(flight_number) = app.submit_input() {
                            app.loading = true;
                            app.last_error = None;

                            let opensky = clients.opensky.clone();
                            let aviationstack = clients.aviationstack.clone();
                            let tx = api_tx.clone();
                            let flight_num = flight_number.clone();

                            tokio::spawn(async move {
                                // Fetch from both APIs in parallel
                                let (position_result, schedule_result) = tokio::join!(
                                    opensky.search_flight(&flight_num),
                                    aviationstack.get_flight(&flight_num)
                                );

                                let _ = tx
                                    .send(ApiResponse::FlightSearch {
                                        flight_number: flight_num,
                                        position: position_result,
                                        schedule: schedule_result.ok().flatten(),
                                    })
                                    .await;
                            });
                        }
                    }
                    KeyCode::Char(c) => {
                        app.input_char(c.to_ascii_uppercase());
                    }
                    KeyCode::Backspace => {
                        app.input_backspace();
                    }
                    KeyCode::Up => {
                        app.history_previous();
                    }
                    KeyCode::Down => {
                        app.history_next();
                    }
                    KeyCode::Esc => {
                        app.mode = AppMode::Viewing;
                        app.input_buffer.clear();
                        app.cursor_position = 0;
                        app.history_index = None;
                    }
                    _ => {}
                }
            }
        }
        AppMode::Viewing => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true;
            }
            KeyCode::Char('/') | KeyCode::Char('a') => {
                app.mode = AppMode::Input;
            }
            KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
            KeyCode::Char('d') => app.remove_selected_flight(),
            KeyCode::Char('r') => {
                if !app.tracked_flights.is_empty() && !app.loading {
                    trigger_refresh(app, clients, api_tx).await;
                }
            }
            _ => {}
        },
    }
}

async fn handle_tick(app: &mut App, clients: &ApiClients, api_tx: mpsc::Sender<ApiResponse>) {
    // Clear error after some time
    if app.last_error.is_some() {
        if let Some(last) = app.last_api_call {
            if last.elapsed().as_secs() > 10 {
                app.last_error = None;
            }
        }
    }

    // Auto-refresh
    if app.should_update() {
        trigger_refresh(app, clients, api_tx).await;
    }
}

async fn trigger_refresh(
    app: &mut App,
    clients: &ApiClients,
    api_tx: mpsc::Sender<ApiResponse>,
) {
    app.loading = true;
    app.last_api_call = Some(Instant::now());
    app.last_error = None;

    for flight in &app.tracked_flights {
        let client = clients.opensky.clone();
        let tx = api_tx.clone();
        let icao24 = flight.icao24.clone();
        let flight_num = flight.flight_number.clone();

        if !icao24.is_empty() {
            tokio::spawn(async move {
                let result = client.get_state(&icao24).await;
                let _ = tx.send(ApiResponse::FlightUpdate(flight_num, result)).await;
            });
        }
    }
}

fn handle_api_response(app: &mut App, response: ApiResponse) {
    app.loading = false;

    match response {
        ApiResponse::FlightSearch {
            flight_number,
            position,
            schedule,
        } => match position {
            Ok(state) => {
                app.add_flight(flight_number, state, schedule);
                app.last_api_call = Some(Instant::now());
            }
            Err(e) => {
                // Even if position failed, we might have schedule data
                if schedule.is_some() {
                    app.add_flight(flight_number, None, schedule);
                    app.last_api_call = Some(Instant::now());
                } else {
                    app.last_error = Some(e.user_message());
                }
            }
        },
        ApiResponse::FlightUpdate(flight_number, result) => match result {
            Ok(state) => {
                app.update_flight(&flight_number, state);
            }
            Err(e) => {
                app.last_error = Some(e.user_message());
            }
        },
    }
}
