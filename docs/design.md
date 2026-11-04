# Design

Living design notes for *Gitche Gumee: The Last Voyage of the Edmund
Fitzgerald*. This file contains spoilers for the hidden ending.

## Decisions

| Question | Decision |
|---|---|
| Can the player survive? | The historical loss is the default. A hidden, rare path survives. |
| Real crew names on the memorial | No, for now. Ask before changing this. |
| Display | 320x200, 16 colors, whole-number scaling. |
| Action layer | Damage model with no score. |
| Saves | None. Runs last one sitting and replay from a seed. |

## Flow

Boot, Title, Port, then Voyage, which hands off to Event panels and Helm
sections as the clock reaches them. Then Outcome and Memorial.

The clock is history: it does not jump, so events keep their real times. Lost
time is paid for in miles not covered.

## Controls

| Screen | Keys |
|---|---|
| Menus and panels | Up/Down to choose, Space or Enter to confirm. Either also finishes typing text. |
| Port | Left/Right changes a setting. |
| Voyage | The clock waits for orders: Up/Down speed, 1-6 pumps, R repair, then Space or Enter to go. It stops again after each helm section, and the hint turns yellow while it waits. Under way, the hint and each helm section's opening line remind the player to keep clear of breakers and debris. The radio log stays historical, so reminders don't go there. Each new radio log line also holds the clock for about 1.8 real seconds so it can be read. |
| Helm | Left/Right or A/D to steer, Up/Down speed. N skips ahead at 12x speed, and again takes the helm back. Everything still happens, and seas the player does not steer around still strike, so a skip cannot dodge the storm or win the survival ending for free. |
| Any screen | P or Esc pauses; P, Esc, Space or Enter resumes. A pause stops game time and sound, and other keys do nothing except mute. M mutes or unmutes, for as long as the game is open. Hints for P and M are always on show in the bottom right corner, and the M hint turns to an amber `M MUTED` while muted. |

## Endings

The ship cannot pass the wreck site before the last contact at 7:20 p.m. At that
moment one test decides the outcome. If it fails, the ship is lost, however
healthy she looks. The ship can also founder earlier if the state of the ship
fails.

To survive, all of these must hold (`survival` in `balance.ron`):

- All hatch clamps secured.
- Kept the Anderson close, asked her to track you, and passed wide of Caribou.
- Sought the lee, or are on the north-shore route.
- Never pushed to keep schedule.
- At the time: water at most 25%, hull at least 50%, list at most 6 degrees, and
  at most one rogue-wave hit.

Both endings have a closing sequence. Neither shows a cause for the loss. The
survival ending and the memorial's bell share one Detroit scene, drawn in
`src/outcome/detroit.rs`.

## Tuning

Everything tunable is in `assets/data/balance.ron`. `pace` is the master game
speed: `0.5` would be half speed. `helm_pace` replaces it during a race. Helm densities and timing are in `helm.ron`.

Whole voyages are played without a window in `src/voyage/balance_tests.rs`.

## Dev tools

With `--features dev`: `DOWNBOUND_AUTOPLAY=careful|careless` plays a run,
`DOWNBOUND_SECTION=<id>` opens a helm section with the player at the wheel, `DOWNBOUND_JUMP=lost|reached` goes to an ending, `DOWNBOUND_SPEED` speeds time
up, `DOWNBOUND_SHOT_DIR` saves screenshots, and `DOWNBOUND_SEED` fixes the seed.

## To do

## Play page console

The touch controls and start screen in `site/play/index.html` are drawn on one
low-resolution canvas, with invisible buttons over it for input and screen readers.

- One thumb steers and the other sets the speed. The player can swap the sides,
  and the choice is kept in browser storage.
- Everything around the game picture uses a dark rust-sepia ramp so that it
  recedes. These colors are outside the game's palette and stay on this page. The
  rule in `ui/palette.rs` still holds for everything the game draws.
- On small browser screens the game scales to fit its frame, even at fractional
  sizes, so that the picture is as large as the layout allows.
- The start screen shows the title card from `tools/gen_title_art.py`. Enter, Space
  or a tap on the card starts the game. On tall phones the buttons below it grow
  to fill the screen.
