# Claude Code Instructions

This file provides context for Claude Code when working on this project.

## Project Overview

Flight Tracker TUI is a terminal user interface (TUI) application for real-time flight tracking. It displays live flight position data and schedule information in an interactive terminal interface.

## Tech Stack

- **Language**: Rust (2021 edition)
- **TUI Framework**: ratatui + crossterm
- **Async Runtime**: tokio
- **HTTP Client**: reqwest
- **APIs**: OpenSky Network (position data), AviationStack (schedule data)

## Project Structure

```
src/
├── main.rs          # Entry point, async event loop
├── app.rs           # Application state and business logic
├── ui.rs            # TUI rendering with ratatui widgets
├── event.rs         # Terminal event handling (keyboard, tick)
├── flight.rs        # Flight and Airport data structures
├── cache.rs         # Generic TTL-based cache
├── history.rs       # Flight history persistence
├── error.rs         # Error types
└── api/
    ├── mod.rs       # API module exports
    ├── opensky.rs   # OpenSky Network client (live position)
    ├── aviationstack.rs  # AviationStack client (schedules)
    └── types.rs     # API response types
```

## Key Concepts

### Data Flow
1. User enters flight number (e.g., UA123)
2. App fetches data from both APIs in parallel
3. OpenSky provides live position (lat/lon, altitude, speed)
4. AviationStack provides schedule (origin, destination, times)
5. Data merged into Flight struct and displayed

### Caching Strategy
- AviationStack: 1 hour TTL (schedules rarely change, limited API quota)
- OpenSky: 10 seconds TTL (position data changes frequently)

### Callsign Normalization
IATA codes (UA, BA) are converted to ICAO callsigns (UAL, BAW) for OpenSky lookup. See `normalize_callsign()` in `opensky.rs`.

## Development Commands

```bash
# Build
cargo build

# Run
cargo run

# Run tests
cargo test

# Run with release optimizations
cargo run --release

# Check for issues
cargo clippy
```

## Environment Variables

- `AVIATIONSTACK_API_KEY` - Required for schedule data (get free key at aviationstack.com)
- `OPENSKY_USERNAME` / `OPENSKY_PASSWORD` - Optional, for higher rate limits

## Code Style

- Use `rustfmt` defaults
- Prefer explicit error handling over `.unwrap()`
- Add doc comments for public APIs
- Keep functions focused and small
- Use `#[allow(dead_code)]` for API response fields that may be used later

## Testing

Unit tests are in each module under `#[cfg(test)]` blocks. Run with `cargo test`.

Key test areas:
- `cache.rs` - TTL expiration, thread safety
- `flight.rs` - Status parsing, struct initialization
- `app.rs` - State management, flight list operations
- `opensky.rs` - Callsign normalization
- `history.rs` - History persistence, deduplication

## Common Tasks

### Adding a new airline code mapping
Edit `normalize_callsign()` in `src/api/opensky.rs`.

### Modifying the UI layout
Edit `draw()` and related functions in `src/ui.rs`.

### Adding new flight data fields
1. Add field to `Flight` struct in `flight.rs`
2. Update `apply_position_data()` or `apply_schedule_data()` in `app.rs`
3. Update UI display in `format_flight_details()` in `ui.rs`

### Changing cache TTL
Edit constants in `opensky.rs` and `aviationstack.rs`.
