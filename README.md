# Prince Toad

A prince has been turned into a toad. Only a kiss can turn him back, so he
sets off, crossing road after road to reach the other side and kiss
whatever is waiting there. A mailbox. A garden gnome. A cow. A cactus (ow).
He kisses a great many strange things before he finds a real princess, and
she turns out to be a toad too.

A lane-crossing arcade game for the terminal, in seven worlds: village
lanes, the highway, the river, the winter road, the night road lit by
headlights, the castle and its moat, and a quiet pond at the end. Plus an
endless road, today's road (the same for everyone today), and two players
at one keyboard, racing or crossing together.

## Controls

- Hop: W A S D or the arrows
- Hop diagonally, in the pond only: Q E Z C, or numpad 7 9 1 3
- Pause: Esc or P. Help: H or F1. Sound: M or F8
- Two players: player one W A S D, player two the arrows
- Gamepad: d-pad or left stick hops (two d-pad directions at once for a
  diagonal), A goes on, Start pauses

On the roads, touch nothing that moves. On the water, stay on logs,
turtles, floes, barrels and ducks (turtles and ducks dive now and then).
A hop onto ice slides one hop further. Lily pads sink a moment after you
land on them.

## Options

```
prince-toad [--endless] [--two] [--together] [--crossing N] [--demo] [--mute]
```

Progress and settings are kept in `$XDG_STATE_HOME/prince-toad/save.txt`
(`~/.local/state/...`), with a session log beside it (`RUST_LOG=debug`
for every key).

## Building

```
cargo build --release
```

Linux needs the ALSA and udev development packages (`libasound2-dev`,
`libudev-dev`).

## Licence

MIT. Made for Asciiverse by David Sandquist, 2026.
