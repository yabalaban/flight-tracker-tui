# Flight Tracker TUI

A terminal-based flight tracker that displays real-time flight information. Track multiple flights simultaneously with live position updates, route information, and schedule data.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Real-time tracking**: Live position data including altitude, speed, and heading
- **Route information**: Origin and destination airports with names
- **Schedule data**: Departure/arrival times with delay information
- **Multi-flight tracking**: Track multiple flights simultaneously
- **Flight history**: Quickly re-track recently searched flights with ↑/↓ keys
- **Keyboard navigation**: Vim-style controls (j/k) plus arrow keys
- **Smart caching**: Minimizes API calls with intelligent TTL-based caching
- **Auto-refresh**: Automatic updates every 30 seconds

## Screenshot

```
┌─ Enter Flight Number (e.g. UA123) ────────────────────────────┐
│ UA100                                                          │
├────────────────────────┬───────────────────────────────────────┤
│ Tracked Flights        │ Flight Details                        │
│ ────────────────────── │ ───────────────────────────────────── │
│ > UA100 SFO→LHR En Route│ Flight:  UA100 (UAL100)              │
│   BA178 LHR→JFK En Route│ Airline: United Airlines             │
│                        │ Status:  En Route                     │
│                        │                                       │
│                        │ Route                                 │
│                        │   From: SFO San Francisco Intl        │
│                        │   To:   LHR London Heathrow           │
│                        │                                       │
│                        │ Schedule                              │
│                        │   Departure:  08:30 (actual: 08:45)   │
│                        │   Arrival:    17:15 (est: 17:30)      │
│                        │                                       │
│                        │ Live Position                         │
│                        │   Position:  51.4700°N, 0.4543°W      │
│                        │   Altitude:  38000 ft                 │
│                        │   Speed:     487 kts                  │
├────────────────────────┴───────────────────────────────────────┤
│ Tracking 2 flight(s) | Next update in 25s | q quit / add      │
└────────────────────────────────────────────────────────────────┘
```

## Installation

### Prerequisites

- Rust 1.70 or later
- An AviationStack API key (free tier available)

### Build from source

```bash
git clone https://github.com/yourusername/flightradar.git
cd flightradar
cargo build --release
```

The binary will be at `target/release/flightradar`.

## Configuration

### Required: AviationStack API Key

Get a free API key at [aviationstack.com](https://aviationstack.com/signup/free) (100 requests/month on free tier).

Set the environment variable:

```bash
export AVIATIONSTACK_API_KEY=your_api_key_here
```

Or create a `.env` file in the project directory:

```
AVIATIONSTACK_API_KEY=your_api_key_here
```

### Optional: OpenSky Network Authentication

For higher rate limits on position data, create a free account at [opensky-network.org](https://opensky-network.org/):

```bash
export OPENSKY_USERNAME=your_username
export OPENSKY_PASSWORD=your_password
```

## Usage

```bash
cargo run
# or if installed
flightradar
```

### Keyboard Controls

| Key | Action |
|-----|--------|
| `/` or `a` | Add a new flight to track |
| `Enter` | Submit flight number |
| `Esc` | Cancel input |
| `↑` | Previous history entry (in input mode) |
| `↓` | Next history entry (in input mode) |
| `j` or `↓` | Select next flight (in view mode) |
| `k` or `↑` | Select previous flight (in view mode) |
| `d` | Delete selected flight |
| `r` | Force refresh all flights |
| `q` | Quit |
| `Ctrl+C` | Quit |

### Flight Number Format

Enter flight numbers in standard format:
- `UA123` - United Airlines flight 123
- `BA285` - British Airways flight 285
- `AF007` - Air France flight 7

The app automatically converts IATA codes to ICAO callsigns for tracking.

## Data Sources

- **[OpenSky Network](https://opensky-network.org/)**: Real-time ADS-B position data (altitude, speed, heading, coordinates)
- **[AviationStack](https://aviationstack.com/)**: Flight schedule data (routes, times, delays, airline info)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Main Event Loop                         │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              tokio::select!                          │    │
│  │   ├── Keyboard Events (crossterm)                   │    │
│  │   ├── Tick Events (250ms interval)                  │    │
│  │   └── API Responses (mpsc channel)                  │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                   App State                          │    │
│  │   • tracked_flights: Vec<Flight>                    │    │
│  │   • selected_index, input_buffer, mode              │    │
│  └─────────────────────────────────────────────────────┘    │
│                           │                                  │
│                           ▼                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │               TUI Rendering (ratatui)               │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Caching Strategy

To minimize API usage:
- **Schedule data** (AviationStack): Cached for 1 hour
- **Position data** (OpenSky): Cached for 10 seconds

## Development

```bash
# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run

# Check for lints
cargo clippy

# Format code
cargo fmt
```

## Project Structure

```
src/
├── main.rs          # Entry point and event loop
├── app.rs           # Application state and logic
├── ui.rs            # Terminal UI rendering
├── event.rs         # Keyboard/terminal event handling
├── flight.rs        # Flight data structures
├── cache.rs         # TTL-based caching
├── history.rs       # Flight history persistence
├── error.rs         # Error types
└── api/
    ├── mod.rs
    ├── opensky.rs       # OpenSky Network client
    ├── aviationstack.rs # AviationStack client
    └── types.rs         # API response types
```

## Limitations

- Position data requires the aircraft to be broadcasting ADS-B
- Some flight numbers may not map correctly to callsigns (e.g., codeshares)
- AviationStack free tier is limited to 100 requests/month
- OpenSky anonymous access is limited to 400 requests/day

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [ratatui](https://github.com/ratatui/ratatui) - Terminal UI framework
- [OpenSky Network](https://opensky-network.org/) - Free ADS-B data
- [AviationStack](https://aviationstack.com/) - Flight schedule API
