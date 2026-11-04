# Gitche Gumee

A retro-style strategy and vertical-scroll sailing game about the final voyage of the SS *Edmund Fitzgerald*, which sank in Lake Superior on November 10, 1975, losing all 29 men aboard. The cause of the wreck is still unknown.

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
| --- | --- |
| Menus and text | Up/Down to choose, Space or Enter to confirm. A press finishes typing text, and the next one moves on. |
| Port | Left/Right changes a setting. |
| Chart | Up/Down for speed, 1–6 for pumps, R to repair, Space or Enter to go. |
| Helm | Left/Right or A/D to steer, Up/Down for speed, N to skip ahead or take the helm back. |
| Any screen | P or Esc to pause or resume (Space or Enter also resumes). M to mute or unmute. |

If no key is pressed for 10 seconds, the game moves on with whatever is selected.

In the browser on a phone or tablet, on-screen buttons take the place of the
keys, and a tap on the game screen acts as Enter. One thumb steers and the other
sets the speed. The start screen and the SWAP button trade the two sides for
left-handed players.

## Build and run

Rust and Bevy support Windows, macOS and Linux, so the game should run on all
three. If you hit a problem,
[open an issue](https://github.com/edwestfieldjr/gitchegumee/issues).

You need Rust 1.97.1 or newer. If you already have Rust, run `rustup update`.

### 1. Install Rust and the system dependencies

**Windows:** download and run `rustup-init.exe` from
[rustup.rs](https://rustup.rs/). If it offers to install the Visual Studio build
tools, accept.

**macOS:** in Terminal, install the Apple command line tools, then Rust:

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Linux:** install Rust with the same `curl` command, then your distribution's
packages:

```sh
# Debian, Ubuntu, Mint, Pop!_OS
sudo apt install g++ pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0 libwayland-dev libxkbcommon-dev

# Fedora
sudo dnf install gcc-c++ libX11-devel alsa-lib-devel systemd-devel wayland-devel libxkbcommon-devel

# Arch, Manjaro
sudo pacman -S --needed alsa-lib systemd-libs vulkan-icd-loader pkgconf

# openSUSE
sudo zypper install libudev-devel gcc-c++ alsa-lib-devel
```

For other distributions, see Bevy's
[Linux dependencies](https://github.com/bevyengine/bevy/blob/main/docs/linux_dependencies.md).

### 2. Get the game and run it

These commands are the same on every system. On Windows, use PowerShell.

```sh
git clone https://github.com/edwestfieldjr/gitchegumee.git
cd gitchegumee
cargo run --release
```

No Git? [Download the ZIP](https://github.com/edwestfieldjr/gitchegumee/archive/refs/heads/main.zip),
unpack it, open a terminal in that folder, and run `cargo run --release`.

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
