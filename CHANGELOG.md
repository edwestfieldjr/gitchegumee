# Changelog

Every change from the original `CLAUDE.md` brief, up to the commit that last
changed this file. `CLAUDE.md` itself has changed in one line: the title.

## Changes you requested after the brief

| Request | What changed |
|---|---|
| New title | **Gitche Gumee: The Last Voyage of the Edmund Fitzgerald** replaces the working title *Downbound* in `CLAUDE.md`, the window title, and the title screen. "Fitzeerald" was corrected to "Fitzgerald". |
| Remove the "in memory" line | The dedication is gone from the title screen and from `text.ron`. |
| Closing sequence for the loss | Side view of the ship in the storm. Her lights fail one by one, the sea rises over the view, and the lake is empty when it falls. Then a descent to the two sections of the wreck at 530 ft, with one bell. |
| Horizontal bars looked like embankments | Waves were flat strips. They are now uneven breakers with whitecaps that taper toward the gap. |
| Slower screens, time to read | Text types out, and Enter finishes it before moving on. A blinds wipe opens each screen. The voyage clock went from 6 to 4 game minutes per second. |
| At least 50% slower | New master setting, `pace` in `assets/data/balance.ron`, set to `0.5`. It slows the whole game clock. |
| Much slower, "a freighter, not a race car" | `pace` went from `0.5` to `0.25`. |
| Racing a little faster | New setting, `helm_pace` in `assets/data/balance.ron`, set to `0.33`. It replaces `pace` during a race, about a third faster than the rest of the game. |
| Full speed 1.75x faster | At the helm, the FULL order scrolls at 2.275x cruise, up from 1.3x. **Read as 1.75x the old full speed**, not 1.75x cruise. The ship's 14 knots on the chart did not change. |
| Time to read, without long delays | Text stays up until a key press moves on, with a 0.5-second guard against double presses. Wipes take about 1 real second. Space does what Enter does on every screen that advances. |
| Less time staring at the map | The chart's clock now waits for speed and pump orders, then Space or Enter runs to the next race in about 5 real seconds at most. The time still passes in the simulation, events still fire, and the chart stops again after each race. |
| Only half the screen wiped | The blinds were centered on the left edge. They now span the view. |
| Survival sequence at Detroit | The ship comes down the river at night, lights on, and sounds her horn, under the caption "DETROIT" in letters a little smaller than the old caption. |
| Use the 1965 skyline, add Mariners' Church | Original pixel art traced by eye from `detroit_skyline_1965.jpg`, with no Renaissance Center, and the church on the shore right of center, drawn from your photos. The church tower is my estimate of about 1.5x true scale, so it can be recognized. No photo ships with the game. |
| Use that scene for the bell | The memorial is now two parts: the words, then the Detroit scene with the 29 tolls, the belfry lighting on each stroke. The game ends on the bell. |
| Turn angle of about 15 degrees | At the helm the ship leans 8 or 15 degrees, not 45. The frames are made by rotating the upright sprite when the game loads. The original 45-degree frames are no longer used. |
| Gitignore `docs/art/reference/` | It was already ignored, and nothing under it is tracked. |
| Breakers should move | Crests heave and sway. Collision follows the sway. |
| Gap widths should vary | Each gap is random, from the old width up to 1.75x. **"175% wider" was read as 175% of the old width**, because 2.75x would leave almost no crest. Change `GAP_WIDEST_FACTOR` in `src/helm/sim.rs` to adjust it. |

The project folder was also renamed twice, to `EdmundFitzgerald` and then
`gitchegumee`. The Cargo package and the `DOWNBOUND_*` dev variables still use
the old name `downbound`.

## Open design decisions, now decided

`CLAUDE.md` still lists these as open. The answers are recorded here and in
`docs/design.md`.

1. **Ending:** the historical loss, with a hidden survival path.
2. **Memorial names:** no real crew names for now.
3. **Display:** 320x200 with 16 colors.
4. **Action layer:** damage model, no score. Hits carry into the voyage.
5. **Saves:** none. Runs are one sitting and can be replayed from a seed.

## Conflicts with the brief, for you to review

- **The new title conflicts with the song rule.** `CLAUDE.md` says not to
  reference the song's lines in text or UI, and "Gitche Gumee" appears in the
  song's opening. The name itself is older (Longfellow, from the Ojibwe). It
  was applied as you asked, and the rule in `CLAUDE.md` was left unchanged.
- **Crate-wide lint allow.** `src/main.rs` allows `too_many_arguments` and
  `type_complexity` across the crate, with a comment saying why, because Bevy
  systems trip both.
- **Some text and facts go beyond the reference section** and should be checked
  against the sources:
  - Weather values between the sourced points are estimates, and are marked as
    such in `weather.ron`.
  - The Anderson warning of traffic ahead at the last contact.
  - The Anderson going back out to search being described in `text.ron`.
  - The shoals north of Caribou Island being called poorly charted.
- **The lake chart is hand-simplified from memory,** not taken from NOAA data.
  It is a game map and is not accurate enough for anything else.
- **Reference photos were committed by mistake, then removed.** Photos dropped
  into `assets/data/` were swept into a docs commit by `git add -A`. That
  unpushed commit was amended to take them out. The files were never lost, and
  are now in `docs/art/reference/`.
- **The skyline is traced from a 1965 photo that may be under copyright.** Only
  shapes were used, redrawn at low resolution, but check this before any public
  release.
- **Early commits do not run.** `CLAUDE.md` asks every step to leave `cargo run`
  working. Commits `feat: add simulation core` through `feat: add title, port`
  compile and pass their tests but crash at startup, because their plugins were
  not registered. `fbb5364` fixed that.

## Choices the brief left open

- **Bevy 0.20.1,** the latest release, since no `Cargo.toml` existed. Default
  features are off in favor of `2d` and `audio`.
- **New crates:** `serde` and `ron`, because Bevy has no built-in RON loading.
  Random numbers use a small built-in PCG32 instead of a crate.
- **Crew:** roles only, with no fictional names.
- **Font:** an original 5x7 uppercase font built in code, so there is no
  `assets/fonts/` folder.
- **Sprites:** your PNGs are unchanged on disk and are snapped to the palette
  when they load.
- **Music:** original, generated by `tools/gen_audio.py` with only the Python
  standard library. Your placeholder theme still plays on the title screen.
- **How the loss is shown:** the ship is never seen breaking or sinking, because
  the cause is unknown. She goes dark, the sea covers the view, and she is gone.

## Added beyond the suggested layout

- `src/boot.rs`, `content.rs`, `rng.rs`, `title.rs`, and `outcome/`.
- `src/devtools.rs`, built only with `--features dev`: screenshots, autoplay,
  and jumping to an ending.
- `tools/gen_audio.py`.
- `assets/data/`: `weather`, `chart`, `events`, `helm`, `balance`, and `text`.
- `docs/design.md`.
