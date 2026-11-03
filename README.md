# Gitche Gumee: The Last Voyage of the Edmund Fitzgerald

A retro, Atari-style game about the last voyage of the SS *Edmund Fitzgerald*
across Lake Superior, November 9–10, 1975.

The *Fitzgerald* sank in a severe storm on November 10, 1975, with the loss of
all 29 men aboard. No bodies were recovered, and the cause of the sinking is
still unknown. This game is meant as a memorial. It refers to the crew by role
rather than by name, and it doesn't present any one theory as fact.

**[Play it and read more at westfield.dev/gitchegumee](https://westfield.dev/gitchegumee/)**

## The game

You command the ship from the dock to the end of the voyage, moving between two
modes:

- **Chart:** give speed and pump orders, then get under way. Radio traffic and
  events come up as the historical clock reaches them, and your decisions carry
  forward.
- **Helm:** steer through top-down scrolling seas, finding the gaps in the waves
  and avoiding debris and shoals through wind and snow.

The two modes feed each other. Your choices at the dock and on the chart shape
the hazards at the helm, and damage and water taken on at the helm stay with the
ship. There is no score and there are no saves: each voyage lasts one sitting.

## Controls

| Screen | Keys |
|---|---|
| Menus and text | Up/Down to choose, Space or Enter to confirm. A press finishes typing text, and the next one moves on. |
| Port | Left/Right changes a setting. |
| Chart | Up/Down for speed, 1–6 for pumps, R to repair, Space or Enter to go. |
| Helm | Left/Right or A/D to steer, Up/Down for speed. |

In the browser on a phone or tablet, on-screen buttons take the place of the
keys, and a tap on the game screen acts as Enter.

## Build and run

You need stable Rust that supports the 2024 edition. The game is developed on
desktop Linux, with a browser build for the web. Windows and macOS builds are
planned but not yet supported.

On Arch Linux, install the system dependencies:

```sh
sudo pacman -S --needed alsa-lib systemd-libs vulkan-icd-loader pkgconf
```

Then clone the repository and run the game:

```sh
git clone https://github.com/edwestfieldjr/gitchegumee.git
cd gitchegumee
cargo run --release
```

The first build compiles [Bevy](https://bevyengine.org/) and takes a while.

To build the website and browser version into `dist/`, install the
`wasm32-unknown-unknown` target and the `wasm-bindgen` version in `Cargo.lock`,
then run `tools/build_web.sh`. Serve `dist/` with any web server.

## Development

```sh
cargo run --features dev                  # faster rebuilds and dev tools
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

The game is written in Rust with Bevy, using one plugin per module under `src/`.
Events, the route, the weather timeline, text and balance are data, kept in RON
files under `assets/data/`. Game speed and most other tuning are in
`assets/data/balance.ron`.

- [`site/`](site/) is the GitHub Pages landing page, deployed by
  [`.github/workflows/pages.yml`](.github/workflows/pages.yml) on pushes to `main`.
- [`docs/design.md`](docs/design.md) has the design notes and dev tools.
  **It contains spoilers.**
- [`CHANGELOG.md`](CHANGELOG.md) records the changes made since the original
  brief.
- [`CLAUDE.md`](CLAUDE.md) has the project brief, historical reference and
  content rules.

## Status

Early and playable from start to finish.

## Content and music

All music is original to this project. The game contains no song lyrics or
commercial recordings, and its charts and maps are original pixel art.

This is an independent project, not affiliated with or endorsed by the families
of the crew, the ship's owners, or any related organization.

## License

Released under the [MIT License](LICENSE).
