# TfL Tracker

A Rust/WASM application for tracking London Underground, DLR, Elizabeth line and Overground trains in real-time.

## Project Structure

This is a Rust workspace with multiple crates:

- `webapp`: The web application built with Yew/WASM
- `stop_points_generator`: A utility to generate station data

## Building and Running

### Generate Station Data (Optional - data is included)

This step is only needed if you want to refresh the station data:

```bash
cd stop_points_generator
cargo run
# This will create stop_points.json in the webapp directory
```

### Run the Web Application

```bash
cd webapp
trunk serve
# Opens the application at http://localhost:8080
```

### Build for Production

```bash
cd webapp
trunk build --release
# Outputs to webapp/dist
```

## Architecture

- The project uses Yew for the frontend framework
- Data is fetched from the TfL API
- The application allows users to:
  - Select a tube station
  - View arriving trains
  - Track specific trains across the network

## Development Notes

- The `stop_points_generator` is a separate binary that can be run standalone
- The web application uses pre-generated station data for faster loading
- All web-specific code is isolated in the `webapp` crate
