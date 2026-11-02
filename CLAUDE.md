# CLAUDE.md

## Project

**Working title:** Downbound

A retro, Atari-2600/7800-style game about the last voyage of the SS *Edmund Fitzgerald*, November 9–10, 1975, Lake Superior. It mixes two modes:

- **Voyage layer (Oregon Trail):** turn/tick-based management across the lake. Prep the ship, pick a route, manage resources, and react to text events and radio traffic.
- **Action layer (Spy Hunter):** top-down vertical-scrolling helm sections. Steer through rising seas, debris, shoals, and snow, keep heading, and survive set-piece hazards.

The player moves between layers along the route. Choices in the voyage layer change difficulty and hazards in the action layer, and action-layer results (hull damage, water taken on, time lost) feed back into the voyage layer.

## Tone and content rules

- This is a real disaster. 29 men died and no bodies were recovered. Families of the crew are still living. Treat it as a memorial, not a joke. No gore, no mockery, no "game over" gags on the sinking.
- **No song lyrics or recordings.** Gordon Lightfoot's 1976 song is inspiration only. Don't quote, paraphrase, or reference its lines in text, UI, filenames, or commit messages. Don't use the audio. Original music only.
- **Crew:** use roles (captain, mates, wheelsman, deckhand, etc.) and fictional names. Don't use the real crew's names in gameplay. A real-name memorial screen is a separate decision — ask before adding one.
- **Ship, route, timeline, weather, and other vessels** can be historical. Keep facts accurate to the reference section below.
- **Cause of sinking is unknown.** Don't present one theory as fact. Hazards can represent several theories (hatch failure, shoaling, topside damage, rogue waves, structural failure).
- **Charts/maps:** draw original pixel art. Use public-domain NOAA chart data for geography if needed. Don't copy commercial chart artwork.

## Stack

- Rust (stable, edition 2024)
- Bevy — use the version in `Cargo.toml`. Before using an API you aren't sure of, check docs.rs for that exact version; Bevy breaks APIs between minor releases. Don't upgrade Bevy without asking.
- Target: desktop Linux first (dev box is Arch). Windows/macOS/WASM later.

### Linux deps (Arch)

```
sudo pacman -S --needed alsa-lib systemd-libs vulkan-icd-loader pkgconf
```

## Commands

```
cargo run                               # dev build
cargo run --features dev                # dev build + dynamic linking + debug overlays
cargo run --release                     # release build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Run `cargo fmt` and `cargo clippy` before calling a task done. Fix warnings; don't silence them with `#[allow]` without a comment saying why.

## Cargo profile

Keep this in `Cargo.toml` for usable dev-build performance:

```toml
[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 3

[features]
dev = ["bevy/dynamic_linking"]
```

## Architecture

- One Bevy `Plugin` per module. `main.rs` only builds the `App` and adds plugins.
- Game flow is a top-level `States` enum, roughly: `Boot → Title → Port → Voyage → Helm → Event → Outcome → Memorial`. Use `SubStates` or `ComputedStates` where a mode has internal phases.
- Despawn per-state entities with `StateScoped` (or the current equivalent) — no manual cleanup lists.
- Simulation (ship state, weather, clock) runs on `FixedUpdate`. Rendering and input on `Update`.
- Seeded RNG resource for all randomness so runs are reproducible. Log the seed on start.
- Content is data, not code: events, waypoints, weather timeline, and radio messages live in RON files under `assets/data/`. Code loads and validates them.
- Keep systems small and single-purpose. Prefer events/messages between plugins over reaching into another plugin's components.

### Suggested layout

```
src/
  main.rs
  states.rs
  ship/        # hull, list, freeboard, water in hold, pumps, radar, crew
  weather/     # wind, wave height, visibility, storm timeline
  voyage/      # route, waypoints, clock, resources, decisions
  helm/        # action layer: scrolling, steering, hazards, collisions
  events/      # text events, radio, choices
  ui/          # HUD, menus, text boxes, retro font
  audio/
assets/        # only files the game loads at runtime; Bevy ships this folder
  sprites/
    freighter/ # one subfolder per subject
  fonts/
  audio/       # .ogg (Bevy can't play .mid)
  data/        # *.ron
docs/
  design.md    # living design doc
  art/         # previews, source frames, zips — not shipped
    reference/ # third-party reference photos — gitignored
  audio/       # .mid sources — not shipped
```

### First task: organize `assets/`

`assets/` currently holds everything flat. Before any code work, sort it into the layout above:

| Current file | Move to |
|---|---|
| `freighter_sprite.png` | `assets/sprites/freighter/freighter_side.png` |
| `freighter_topdown_sheet.png` | `assets/sprites/freighter/` (640×80, 8 frames of 80×80, order N NE E SE S SW W NW) |
| `title_theme_placeholder.ogg` | `assets/audio/` |
| `title_theme_placeholder.mid` | `docs/audio/` |
| `freighter_topdown_preview_4x.png` | `docs/art/` |
| `freighter_topdown_8dir.zip` | `docs/art/` |
| `edmondfitzgerald.jpg` | `docs/art/reference/` |
| `freighter_topdown_{N,NE,E,SE,S,SW,W,NW}.png` | delete (duplicated in the sheet and the zip) |

Then:
- Add `docs/art/reference/` to `.gitignore`.
- Run `find assets docs -type f | sort` and show the result.
- Don't move or delete anything not in this table; ask instead.

Going forward, put new runtime files in the matching `assets/` subfolder and everything else (previews, sources, references) under `docs/`.

## Visual and audio style

- Low internal resolution (start at 320×200), integer-scaled to the window. Nearest-neighbor sampling (`ImagePlugin::default_nearest()`), no subpixel jitter.
- Limited palette, Atari-inspired. Define it once in `ui/palette.rs` and use only those colors.
- Chunky bitmap font. Text events render in boxed panels like an 8-bit adventure game.
- Chiptune/PSG-style original music and SFX. Storm intensity drives the music layer.

## Core systems (first pass)

**Ship state:** hull integrity, water in hold, list (degrees, port/starboard), freeboard, speed, heading, fuel, pumps (6 total, each up/down), radar (2 units), crew fatigue/morale.

**Weather:** driven by a timeline table (see reference). Wind, wave height, wind direction, and visibility change over the clock. Rogue-wave sets ("three sisters": three waves in a row, each larger than normal) as scripted hazards in the last stretch.

**Voyage decisions (examples):**
- How many hatch clamps to secure before departure (time vs. safety).
- Route: standard Lake Carriers' Association downbound track vs. the north-shore route (the route the *Wilfred Sykes* took).
- Slow down vs. push to keep schedule.
- Seek lee near Isle Royale or the Ontario shore.
- Keep pace with the *Arthur M. Anderson* for radar guidance after radar failure.
- Run for Whitefish Bay vs. alternatives.

**Helm hazards (examples):** waves breaking over deck, floating logs/debris, shoal zones near Caribou Island, snow squalls cutting visibility, wind shifts that change steering, loss of radar (minimap goes dark), unlit navigation beacon.

## Historical reference

Research sources — check these before adding new facts or drawing the ship:

- Wikipedia, "SS Edmund Fitzgerald": https://en.wikipedia.org/wiki/SS_Edmund_Fitzgerald — main source for this section. The infobox has a 1971 photo of the ship; use it as the visual reference for the ship sprite (classic laker profile: pilothouse forward, long flat hatch deck, engine house and stack aft).
- NTSB Marine Accident Report MAR-78-03 (1978): https://www.ntsb.gov/investigations/AccidentReports/Reports/MAR7803.pdf — ship diagrams, hatch layout, track charts, weather data.
- USCG Marine Board Casualty Report (1977), cited in the Wikipedia article.

Reference photos are for drawing original pixel art only. Don't copy photos or chart images into `assets/`.

**Ship:** Great Lakes freighter, launched 1958. 729 ft long, 75 ft beam, ~25 ft draft. 21 cargo hatches, 3 holds divided by non-watertight screen bulkheads. 6 bilge pumps. Top speed ~14 kn. Crew 29. No fathometer (hand line only). Reduced-freeboard load line meant about 11.5 ft of deck above water in the storm.

**Cargo:** 26,116 long tons of taconite pellets, bound for Zug Island near Detroit.

**Timeline (local time):**

| When | What |
|---|---|
| Nov 9, 2:15 p.m. | Departs Superior, Wisconsin |
| Nov 9, ~5 p.m. | Joins *Arthur M. Anderson* (from Two Harbors, MN) |
| Nov 9, 7 p.m. | NWS issues gale warnings for all of Lake Superior; both ships turn north toward the Ontario shore |
| Nov 10, 1 a.m. | Storm hits; 52 kn winds, 10 ft waves reported |
| Nov 10, 2 a.m. | Warnings upgraded to storm |
| Nov 10, ~3 a.m. | *Fitzgerald* pulls ahead of *Anderson* |
| Nov 10, afternoon | Storm center passes; winds shift NE → S → NW, then build again |
| Nov 10, 2:45 p.m. | Snow; *Anderson* loses sight of *Fitzgerald* (~16 mi ahead) |
| Nov 10, ~3:30 p.m. | Reports fence rail and two vent covers lost, taking on water, starboard list; two pumps running |
| Nov 10, ~3:35 p.m. | USCG closes the Soo Locks, tells ships to seek anchorage |
| Nov 10, ~4:10 p.m. | Reports radar failure; asks *Anderson* to track them |
| Nov 10, 4:39 p.m. | Whitefish Point light and radio beacon reported out |
| Nov 10, 5–5:30 p.m. | *Avafors* reports the light on, beacon off; *Fitzgerald* reports bad list, both radars lost, heavy seas over deck |
| Nov 10, ~6 p.m. | Waves up to 25 ft; gusts 70–75 kn; rogue waves to ~35 ft |
| Nov 10, 7:10 p.m. | Last radio contact — captain reports they're holding their own |
| Nov 10, ~7:20 p.m. | Gone from radar and radio. No distress call |

**Wreck site:** 46°59′54″N 85°06′36″W, Canadian waters near the boundary, 530 ft deep, about 17 mi (15 nmi) from the entrance to Whitefish Bay. Two main sections on the bottom.

**Other vessels:** *Arthur M. Anderson* (following ship, gave radar guidance, later searched), *Wilfred Sykes* (took the north-shore route and sheltered at Thunder Bay), *Avafors* (reported on Whitefish Point aids), *William Clay Ford* (joined the search).

**Aftermath (for the memorial/outcome screen):** Mariners' Church in Detroit rang its bell 29 times. The ship's bell was recovered in 1995 and is at the Great Lakes Shipwreck Museum, Whitefish Point. The loss led to required depth finders, survival suits, better charts, revised wave forecasting, and annual fall USCG inspections.

## Open design decisions

Ask before deciding any of these:

1. Can the player survive (counterfactual ending), or is the historical outcome fixed?
2. Memorial screen with real crew names — yes/no.
3. Exact internal resolution and palette size.
4. Win/lose and scoring model for the action layer.
5. Save system (none, per-run, or checkpoints).

## Working rules

- Plan before large changes: outline the approach, then build in small, runnable steps. Every step should leave `cargo run` working.
- One feature per commit. Conventional commit messages (`feat:`, `fix:`, `refactor:`, `docs:`).
- Add unit tests for simulation logic (ship state, weather timeline, event conditions). Rendering doesn't need tests.
- Update `docs/design.md` when a design decision lands.
- Don't add crates without saying why. Prefer Bevy built-ins.
- Placeholder art is fine (colored rects). Don't block gameplay work on art.
