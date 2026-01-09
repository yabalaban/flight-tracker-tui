use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode};
use crate::flight::{Flight, FlightStatus};

pub fn draw(frame: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_input(frame, main_chunks[0], app);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_chunks[1]);

    draw_flight_list(frame, content_chunks[0], app);
    draw_flight_details(frame, content_chunks[1], app);
    draw_status_bar(frame, main_chunks[2], app);
}

fn draw_input(frame: &mut Frame, area: Rect, app: &App) {
    let style = if app.mode == AppMode::Input {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if app.mode == AppMode::Input {
        if app.history_index.is_some() {
            " History (↑/↓ to browse) "
        } else if !app.history.is_empty() {
            " Enter Flight Number (↑ for history) "
        } else {
            " Enter Flight Number (e.g. UA123) "
        }
    } else {
        " Press '/' to add flight "
    };

    let input = Paragraph::new(app.input_buffer.as_str())
        .style(style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(style),
        );

    frame.render_widget(input, area);

    if app.mode == AppMode::Input {
        frame.set_cursor_position((area.x + app.cursor_position as u16 + 1, area.y + 1));
    }
}

fn draw_flight_list(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .tracked_flights
        .iter()
        .enumerate()
        .map(|(i, flight)| {
            let is_selected = Some(i) == app.selected_index;

            let status_color = status_to_color(&flight.status);
            let prefix = if is_selected { "> " } else { "  " };

            // Build route string
            let route = match (&flight.origin, &flight.destination) {
                (Some(orig), Some(dest)) => {
                    let orig_code = orig.iata.as_deref().or(orig.icao.as_deref()).unwrap_or("???");
                    let dest_code = dest.iata.as_deref().or(dest.icao.as_deref()).unwrap_or("???");
                    format!(" {}→{}", orig_code, dest_code)
                }
                _ => String::new(),
            };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::styled(&flight.flight_number, Style::default().fg(Color::White)),
                Span::styled(route, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("{}", flight.status), Style::default().fg(status_color)),
            ]);

            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Tracked Flights "),
    );

    frame.render_widget(list, area);
}

fn status_to_color(status: &FlightStatus) -> Color {
    match status {
        FlightStatus::EnRoute => Color::Green,
        FlightStatus::Scheduled => Color::Cyan,
        FlightStatus::Landed => Color::Blue,
        FlightStatus::OnGround => Color::Blue,
        FlightStatus::Delayed => Color::Yellow,
        FlightStatus::Cancelled => Color::Red,
        FlightStatus::NotFound => Color::Red,
        FlightStatus::Unknown => Color::DarkGray,
    }
}

fn draw_flight_details(frame: &mut Frame, area: Rect, app: &App) {
    let flight = app
        .selected_index
        .and_then(|i| app.tracked_flights.get(i));

    let content = match flight {
        Some(f) => format_flight_details(f),
        None => format_empty_state(app),
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Flight Details "),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

fn format_flight_details(flight: &Flight) -> Vec<Line<'static>> {
    let mut lines = vec![];

    lines.push(Line::from(""));

    // Flight number and callsign
    let mut flight_line = vec![
        Span::styled("Flight:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(flight.flight_number.clone()),
    ];
    if !flight.callsign.is_empty() {
        flight_line.push(Span::styled(
            format!(" ({})", flight.callsign),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines.push(Line::from(flight_line));

    // Airline
    if let Some(airline) = &flight.airline {
        lines.push(Line::from(vec![
            Span::styled("Airline: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(airline.clone()),
        ]));
    }

    // Status
    let status_color = status_to_color(&flight.status);
    let mut status_line = vec![
        Span::styled("Status:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}", flight.status), Style::default().fg(status_color)),
    ];
    if let Some(delay) = flight.departure_delay.or(flight.arrival_delay) {
        if delay > 0 {
            status_line.push(Span::styled(
                format!(" (+{}min)", delay),
                Style::default().fg(Color::Yellow),
            ));
        }
    }
    lines.push(Line::from(status_line));

    // Route section
    if flight.origin.is_some() || flight.destination.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Route",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )));

        if let Some(orig) = &flight.origin {
            let code = orig.iata.as_deref().or(orig.icao.as_deref()).unwrap_or("???");
            let name = orig.name.as_deref().unwrap_or("");
            lines.push(Line::from(format!("  From: {} {}", code, name)));
        }

        if let Some(dest) = &flight.destination {
            let code = dest.iata.as_deref().or(dest.icao.as_deref()).unwrap_or("???");
            let name = dest.name.as_deref().unwrap_or("");
            lines.push(Line::from(format!("  To:   {} {}", code, name)));
        }
    }

    // Schedule section
    let has_schedule = flight.departure_scheduled.is_some() || flight.arrival_scheduled.is_some();
    if has_schedule {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Schedule",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )));

        if let Some(dep) = &flight.departure_scheduled {
            let time = format_time(dep);
            let mut dep_line = format!("  Departure:  {}", time);
            if let Some(actual) = &flight.departure_actual {
                dep_line.push_str(&format!(" (actual: {})", format_time(actual)));
            } else if let Some(est) = &flight.departure_estimated {
                dep_line.push_str(&format!(" (est: {})", format_time(est)));
            }
            lines.push(Line::from(dep_line));
        }

        if let Some(arr) = &flight.arrival_scheduled {
            let time = format_time(arr);
            let mut arr_line = format!("  Arrival:    {}", time);
            if let Some(actual) = &flight.arrival_actual {
                arr_line.push_str(&format!(" (actual: {})", format_time(actual)));
            } else if let Some(est) = &flight.arrival_estimated {
                arr_line.push_str(&format!(" (est: {})", format_time(est)));
            }
            lines.push(Line::from(arr_line));
        }
    }

    // Position section (only if we have live data)
    if flight.latitude.is_some() || flight.altitude_ft.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Live Position",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )));

        if let (Some(lat), Some(lon)) = (flight.latitude, flight.longitude) {
            let lat_dir = if lat >= 0.0 { "N" } else { "S" };
            let lon_dir = if lon >= 0.0 { "E" } else { "W" };
            lines.push(Line::from(format!(
                "  Position:  {:.4}°{}, {:.4}°{}",
                lat.abs(), lat_dir, lon.abs(), lon_dir
            )));
        }

        if let Some(alt) = flight.altitude_ft {
            lines.push(Line::from(format!("  Altitude:  {:.0} ft", alt)));
        }

        if let Some(hdg) = flight.heading {
            lines.push(Line::from(format!("  Heading:   {:.0}°", hdg)));
        }

        if let Some(gs) = flight.ground_speed_kts {
            lines.push(Line::from(format!("  Speed:     {:.0} kts", gs)));
        }

        if let Some(vr) = flight.vertical_rate {
            let vr_str = if vr >= 0.0 {
                format!("+{:.0}", vr)
            } else {
                format!("{:.0}", vr)
            };
            lines.push(Line::from(format!("  Climb:     {} ft/min", vr_str)));
        }
    }

    // Aircraft info
    if flight.aircraft_type.is_some() || flight.registration.is_some() || !flight.icao24.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Aircraft",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )));

        if let Some(aircraft) = &flight.aircraft_type {
            lines.push(Line::from(format!("  Type:      {}", aircraft)));
        }

        if let Some(reg) = &flight.registration {
            lines.push(Line::from(format!("  Reg:       {}", reg)));
        }

        if !flight.icao24.is_empty() {
            lines.push(Line::from(format!("  ICAO24:    {}", flight.icao24)));
        }

        if let Some(squawk) = &flight.squawk {
            lines.push(Line::from(format!("  Squawk:    {}", squawk)));
        }
    }

    // Not found message
    if flight.status == FlightStatus::NotFound && flight.origin.is_none() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No data available for this flight.",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from("The flight may not be active or"));
        lines.push(Line::from("the flight number may be incorrect."));
    }

    // Last updated
    if let Some(updated) = flight.last_updated {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Updated: {}", updated.format("%H:%M:%S UTC")),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines
}

fn format_empty_state(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![];

    lines.push(Line::from(""));

    // Show history if available
    if !app.history.is_empty() {
        lines.push(Line::from(Span::styled(
            "Recent Flights",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )));
        lines.push(Line::from(""));

        for (i, entry) in app.history.entries().take(8).enumerate() {
            let route_str = entry
                .route
                .as_ref()
                .map(|r| format!(" {}", r))
                .unwrap_or_default();

            let style = if app.history_index == Some(i) {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(entry.flight_number.clone(), style),
                Span::styled(route_str, Style::default().fg(Color::DarkGray)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press ↑ in input to cycle through history",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "No flight selected",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("Enter a flight number above to start tracking."));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Controls:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from("  /     - Add a new flight"));
    lines.push(Line::from("  ↑/↓   - Browse history (in input)"));
    lines.push(Line::from("  j/k   - Navigate flights"));
    lines.push(Line::from("  d     - Remove selected flight"));
    lines.push(Line::from("  r     - Force refresh"));
    lines.push(Line::from("  q     - Quit"));

    lines
}

fn format_time(time_str: &str) -> String {
    // Parse ISO 8601 time and format nicely
    // Input: "2024-01-15T14:30:00+00:00"
    // Output: "14:30"
    if let Some(t_pos) = time_str.find('T') {
        let time_part = &time_str[t_pos + 1..];
        if time_part.len() >= 5 {
            return time_part[..5].to_string();
        }
    }
    time_str.to_string()
}

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status = if let Some(err) = &app.last_error {
        Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        ))
    } else if app.loading {
        Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::Yellow),
        ))
    } else if let Some(msg) = &app.status_message {
        Line::from(Span::styled(msg.clone(), Style::default().fg(Color::Cyan)))
    } else {
        let update_info = if let Some(secs) = app.seconds_until_update() {
            format!(" | Next update in {}s", secs)
        } else {
            String::new()
        };

        Line::from(vec![
            Span::raw(format!(
                "Tracking {} flight(s){}",
                app.tracked_flights.len(),
                update_info
            )),
            Span::raw(" | "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" add  "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" delete  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" refresh"),
        ])
    };

    let status_bar = Paragraph::new(status).block(Block::default().borders(Borders::ALL));

    frame.render_widget(status_bar, area);
}
