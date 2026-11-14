# Prince Toad

Our own game from David's story, in the lane-crossing genre. Rust +
Ratatui, MIT, repo theasciiverse/prince-toad (private until David's
go-public step). Asciiverse house rules apply (the Asciiverse and games
CLAUDE.md files): no em-dashes or en-dashes anywhere, store text describes
the game and nothing defensive, commits as `David <sandquist@gmail.com>`
ending with the Co-Authored-By line, versions 0.x.0 while unfinished.

**Status:** built overnight on rover, 2026-11-13/14, carried on
2026-11-15. Tagged v0.5.0 (the whole story, first road to the pond),
v0.6.0 (trains, flies, today's road, two players together, livelier kiss
scenes, world tunes), v0.7.0 (fog and plank crossings on the endless
road) and v0.8.0 (options, high-score tables, the kiss scenes' moment of
hope). v0.7.0 and later are tagged but have no release builds: GitHub
Actions refuses jobs on the private theasciiverse repos over account
billing. Nobody has played it yet but the solver and the demo; speeds and
clocks want a human hand (ROADMAP.md). Waiting on David: the name check
below, going public (hub's `tools/go-public.sh --run`, then re-push the
newest tag), and his own play-through before 1.0.0.

## David's notes (copied from games/prince-toad/CLAUDE.md on the main machine)

**Story (David):** a prince has been turned into a toad. Only a kiss can
turn him back, so he sets off, desperately crossing road after road to reach
the other side and kiss whatever is waiting there. He kisses so many weird
things before he finds a real princess. And she turns out to be another
toad. He becomes her prince, and they live happily ever after. He stays a
toad: that is the happy ending.

**Genre:** the lane-crossing arcade game (traffic lanes, then a river of
floating logs and diving turtles), first made famous by Konami's 1981
coin-op. The mechanic is free; its name, art and levels are not. Never call
it by that game's name or name it in store text.

**Decided:** the last level is a quiet pond with no traffic, but still a
challenge to hop the lily pads (David). Suggested and fitting: pads sink a
moment after he lands, drift on a slow current, and the princess hops away
playfully so he has to meet her; falling in may just be a splash back to
the bank. Diagonal hops: four directions on the roads (diagonals would cut
through the timing gaps); diagonals only in the pond, as the finale's one
new move, 2 columns by 1 row (cells are twice as tall as wide); keyboard
Q/E/Z/C or numpad 7/9/1/3, d-pad diagonals on a pad.

**Ideas to build:** each crossing ends in a kiss with a short funny scene
(a mailbox, a garden gnome, a cow, a lamppost, a snowman, a cactus (ow), a
sleeping dragon, a fish, a princess statue, a cardboard cut-out, a duchess
who screams); the world changes level group by level group (village lanes
with carts, highway, river, winter road, castle moat, a night road lit by
headlights); the final scene is two toads and a tiny crown. Smooth lane
scrolling with half blocks, a whole-cell hop, sound made in code with a
croak per hop, endless mode, two players at one keyboard. Gamepad.

**Name check still open:** Steam store search and USPTO for "Prince Toad",
and whether Nintendo's marks for its "Toad" character matter for a title
with the plain word; nothing found by web search so far.

## Name check, 2026-11-13 (web search only; David decides)

"Prince Toad" stays the working title. Two web searches on rover:

- Steam: no game called Prince Toad. Near neighbours: *The Frog Prince*
  (app 2208130, a cursed frog prince, coming soon), *Road Toad* (an arcade
  toad game, Early Access), *A toad well travelled*, *Toads of the Bayou*.
  *Road Toad* is the closest in theme; worth a look before going public.
- Trademarks: no "Prince Toad" mark found through Justia and similar
  indexes. Unrelated marks with the word: TOAD & TADPOLE (toys), TOAD
  AGAIN (Toad & Co. clothing). The USPTO's own search (tmsearch.uspto.gov)
  was not run; Nintendo's Toad character marks not checked directly.
- The game uses nothing of Nintendo's Toad: our toad is a plain toad
  (two quarter-block cells with eyes), no mushroom cap, no character look.

## What is built (0.5.0)

- **Story:** seven worlds, eighteen crossings, each ending in a kiss scene
  (mailbox, garden gnome, cow, cardboard princess, cactus, traffic cone,
  fish, rubber duck, heron, snowman, icicle on the well, lost mitten,
  lamppost, the moon in a puddle, scarecrow, princess statue, duchess,
  sleeping dragon), then the Quiet Pond and the finale (two toads, a tiny
  crown). A three-page prologue on a new story; world cards between worlds.
- **Worlds:** the Village Lanes (carts, bicycles, geese, tractors, hedges),
  the Highway (cars, trucks, motorbikes, cacti), the River (logs, turtles
  that dive), the Winter Road (sleds, snowplows, skidding cars, ice that
  slides you one more hop, ice floes that crack), the Night Road (dark, lit
  by headlights and lampposts), the Castle (guards, knights, carriages, a
  moat of barrels and diving ducks, the dragon's snoring fire), the Quiet
  Pond (no clock, lily pads that sink a moment after you land and come
  back, drifting rows, diagonals, the princess hopping away three times).
- **Endless road:** generated levels, harder as you go, through the
  worlds in turn; each one checked with the solver before it is played.
  Best score saved.
- **Two players:** a race on generated roads, W A S D against the arrows
  (or two gamepads); first to five crossings wins. Or together: both on the
  endless road, a road is done when every toad with lives left is across.
- **Today's road** (0.6): the endless road from a seed of the date, with
  its own best score. Trains with a bell and blinking lights (rail lanes),
  flies to eat for 200.
- Lives (5, extra every 5000), a clock per crossing, score (10 a new row,
  50 plus 10 a second left per crossing, 1000 for the princess). Game over
  lets you retry the crossing with fresh lives and half the score.
- Sound all made in code (`audio.rs`): three croaks taking turns, splat,
  splash, plop, kiss (breath, whistle, pop), jingles. `--mute`, M or F8.
- Gamepad through gilrs (`input.rs`).
- Save and settings in `$XDG_STATE_HOME/prince-toad/save.txt`, session log
  `session.log` beside it (`RUST_LOG=debug` for every key and a snapshot
  every second).
- 80x24 smallest, drawn for 100x30.
- **Options** (0.8): pace (normal, gentle 0.75, slow 0.5: `Game::pace`
  scales the main loop's clock during play only, so the toad, traffic and
  the crossing clock all slow together), lives (5, 9, never run out),
  sound (on, quiet, off), keys for the eight hops (`input::ACTIONS`,
  saved as `key=p1_up:i`; chosen keys are looked at before the usual
  ones, which keep working). Pace also in the pause menu.
- **High scores** (0.8): a table of 8 each for the story, the endless
  road and today's road (`save::Entry`, `score=` lines). A game offers
  its score at game over, at the end of the finale and when quitting to
  the title; a new line asks for a name, a retried story keeps its one
  line (`run`). Scores made with a slower pace or more lives carry a tag.
- **Kiss scenes** (0.8): smack, then sparkles gather round him
  (`KISS_HOPE`) and fizzle (`KISS_FIZZLE`), with their own sounds; then
  the story, then a verdict (`levels::VERDICTS`), and a tally of kisses
  and princes. A test checks every scene fits at 100x30 and 80x24.

## How it works

- `world.rs`: lanes are one row each. Everything on a lane moves together
  and is a pure function of time; a pattern string per lane (each run of a
  letter is one thing, `.` empty, repeated to 100 columns, the field is
  80 wide with 10 columns of lane on each side). The toad hops 2 columns or
  1 row, at most one hop every 8 ticks (60 ticks a second), rides
  floaters, slides on ice. `Toad::tick` is the single rule book.
- `levels.rs`: the story, its levels written top (far bank) to bottom
  (start), the kisses with their art and lines, prologue and finale.
- `solver.rs`: breadth-first search over hops with the same `Toad::tick`.
  The tests prove every crossing can be made in well under its time, the
  pond included; it plays the demo; it vets every endless level.
- `endless.rs`: the generator.
- `game.rs`: phases, modes, lives, score, the princess.
- `render.rs`: cells drawn by hand into the buffer; moving bodies have
  half-block ends so they glide half a column at a time; sky, clouds and
  skyline per world; the night's light map.
- `input.rs`, `audio.rs`, `save.rs`, `main.rs` on tspace's pattern.

## Playtesting

Light, by feel (house rule): the solver proves crossings are possible and
not tight; speeds and gaps are tuned by watching. Scripted play runs in
tmux with `XDG_STATE_HOME` pointed at a scratch folder in the same
command, never the real save. Tests never see a state folder
(`save::state_dir` is None under `cfg(test)`).

## Commands

```bash
systemd-run --user --scope -p MemoryMax=4G cargo build --release -j4
systemd-run --user --scope -p MemoryMax=4G cargo test -j4
target/release/prince-toad                 # title
target/release/prince-toad --crossing 19   # straight to the pond
target/release/prince-toad --demo --crossing 7
asciiverse/record-preview.sh               # the 100x30 catalog cast
```

## Release

Tag `vX.Y.Z` matching Cargo.toml; `.github/workflows/release.yml` builds
Linux (glibc, needs libasound2 and libudev1), macOS and Windows archives
with SHA256SUMS, notes from CHANGELOG.md. The repo is private until David
runs hub's `tools/go-public.sh --run`, then re-pushes the tag.
