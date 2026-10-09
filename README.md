# TfL Tracker

A Rust/WASM application for tracking London Underground, DLR, Elizabeth line and Overground trains in real-time.

## Project Structure

This is a Rust workspace with multiple crates:

- `webapp`: The web application built with Yew/WASM
- `stop_points_generator`: A utility to generate station data
- `station_names`: Station name normalisation shared by both

## Building and Running

### Generate Station Data (Optional - data is included)

This step is only needed if you want to refresh the station data:

```bash
cargo run -p stop_points_generator
# Writes webapp/stop_points.json
```

The generator groups stop IDs into stations by TfL hub code and by
normalised name, and warns about names that may need a new rule in
`station_names`.

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

To serve from a sub-path, pass it with `--public-url`, e.g.
`trunk build --release --public-url /tfl_tracker/`.

### Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p tfl_tracker --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
```

## Deployment

The `CI` workflow runs the checks above on every push and pull request, and
deploys the site to GitHub Pages on pushes to `main`. To enable it, set
**Settings → Pages → Build and deployment → Source** to **GitHub Actions**.

## Architecture

- The project uses Yew for the frontend framework
- Data is fetched from the TfL API
- The application allows users to:
  - Select a station (the URL hash keeps it, so it can be bookmarked or shared)
  - View arriving trains, refreshed every 30 seconds
  - Track specific trains across the network (saved across reloads)
- A service worker caches the app for offline use; live data always comes
  from the network

## Development Notes

- The `stop_points_generator` is a separate binary that can be run standalone
- The web application uses pre-generated station data for faster loading
- All web-specific code is isolated in the `webapp` crate
