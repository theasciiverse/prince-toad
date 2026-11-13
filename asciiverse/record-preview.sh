#!/bin/sh
# Records the catalog preview, asciiverse/prince-toad.cast: 100x30,
# `prince-toad --demo --crossing 8` (the toad crosses the river by itself,
# kisses a rubber duck, crosses the rapids, kisses a heron, and on into
# the winter road), quit with a key after about 55 seconds.
# Runs asciinema in a detached tmux pane, so nothing from this terminal gets
# in; a throwaway HOME and state folder keep the save and log out of the way.
#
#   cargo build --release && asciiverse/record-preview.sh
set -eu
cd "$(dirname "$0")/.."
out=asciiverse/prince-toad.cast
tmp=$(mktemp -d)
trap 'tmux -L pt-rec kill-server 2>/dev/null || true; rm -rf "$tmp"' EXIT
rm -f "$out"
tmux -L pt-rec new-session -d -s rec -x 100 -y 30 \
  "env -i PATH=$PWD/target/release:/usr/bin:/bin:/snap/bin HOME=$tmp XDG_STATE_HOME=$tmp/state USER=player LOGNAME=player \
     TERM=xterm-256color COLORTERM=truecolor LANG=C.UTF-8 \
   asciinema rec --window-size 100x30 --idle-time-limit 2 \
     --capture-env TERM,COLORTERM,LANG --title 'Prince Toad' \
     --command 'prince-toad --demo --crossing 8 --mute' $out"
sleep 56
tmux -L pt-rec send-keys -t rec Enter
sleep 2
ls -la "$out"
