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
| Menus and panels | Up/Down to choose, Enter to confirm. Enter also finishes typing text. |
| Port | Left/Right changes a setting. |
| Voyage | Up/Down speed, 1-6 pumps, R repair, hold Space to run faster. |
| Helm | Left/Right or A/D to steer, Up/Down speed. |

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
speed: `0.5` is half speed. Helm densities and timing are in `helm.ron`.

Whole voyages are played without a window in `src/voyage/balance_tests.rs`.

## Dev tools

With `--features dev`: `DOWNBOUND_AUTOPLAY=careful|careless` plays a run,
`DOWNBOUND_JUMP=lost|reached` goes to an ending, `DOWNBOUND_SPEED` speeds time
up, `DOWNBOUND_SHOT_DIR` saves screenshots, and `DOWNBOUND_SEED` fixes the seed.

## To do

- Replace the placeholder title theme.
