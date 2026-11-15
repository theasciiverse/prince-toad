# Prince Toad roadmap

## 0.5.0 (2026-11-14)

The whole story, first road to the pond: seven worlds, eighteen crossings
and kisses, the Quiet Pond with sinking lily pads, diagonals and the
princess, the finale. Endless road and a two-player race. Sound in code,
gamepad, save, demo.

## 0.6.0 (2026-11-14)

Trains at railway crossings, flies to eat, today's road, two players
together, livelier kiss scenes, a tune per world, tighter clocks.

## 0.7.0 (2026-11-14)

Fog and plank crossings on the endless road; small fixes.

## 0.8.0 (2026-11-15)

Options (pace, lives, sound, keys), high-score tables with names, the
kiss scenes' moment of hope, a gentler step up to the highway.

## 0.9.0 (2026-11-15)

The waiting kiss on the horizon, a drawbridge, market stalls, rain on
the endless road, lines for every squash, the way so far on the world
cards, a finale that moves.

## 1.0.0 (2026-11-15)

Played through and tuned on rover, then tagged on David's go-ahead to
tag 1.0.0 once a game is thoroughly played and tuned:

- **Played through:** the demo crossed all eighteen roads in tmux from
  Mill Lane to the Dragon's Doorstep; a test plays the pond, chasing the
  princess on sinking pads with diagonals, to the finale and every page
  of it to the title; the endless road, game over, a name in the table
  and the records were played with keys in tmux, at 100x30 and 80x24.
- **Tuned:** a short-sighted player (`solver::human`, decides four times
  a second, looks three hops ahead and judges the traffic a little late)
  crossed every road 24 times. The spikes it found were eased: the fast
  bicycles on Pasture Gate (6.5 to 5.5), the sleds on Mitten Pass (7 to
  6), the dragon's fire on the Dragon's Doorstep (14 and 16 to 12 and
  13). The Doorstep stays the hardest road, as the last before the pond.
- **Rain in a story crossing:** Moon Puddle.
- **Endless:** a record per world, the quickest crossing, kept at the
  normal pace.

## 1.1.0 (2026-11-15)

- **Boss roads:** four tall crossings set between the worlds, 21 to 28
  rows against 9 to 13 for a normal road, with a camera that rolls up
  after the toad, a chase from below (steamroller, flood, crows, the
  dragon) that starts after a short grace and climbs at 0.6 to 0.65 rows
  a second, and one or two resting banks that become the respawn point.
  The Long Haul (after the Village Lanes), The Thaw (after Mitten Pass),
  The Long Night (after Crow Field), The Dragon Wakes (after the
  Doorstep, before the pond). Kisses 23, verdicts 22.
- **Tuned** with `solver::human` (eye 6 and 12 ticks late, 24 tries):
  deaths per try 1.4/2.4 on The Long Haul, 0.7/1.3 on The Thaw, 1.3/2.1
  on The Long Night, 1.6/4.0 on The Dragon Wakes, all crossed in 12 to
  21 seconds of 70 to 80. The Dragon Wakes' first moat was eased after
  it caused most deaths. Chases first ran at 0.45 and never caught
  anyone; 0.6 makes them felt.
- **Cameos** from Asciiverse's own games, and the goldfish after Ondřej
  Žára's Goldfish (David, 2026-11-15).
- **Save:** `story=2` marks the new numbering; an older save is moved
  past the boss roads it had already passed.

## Still to come

- **David's own play-through** by hand. Speeds, gaps, clocks, the pond's
  sinking time and the lives were tuned by a scripted player, not by
  feel; his hand may still move them.
