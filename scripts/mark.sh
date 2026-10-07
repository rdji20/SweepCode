#!/bin/sh
# usage: scripts_mark.sh <state x|~| > <id>...   marks items in STATUS.md
state="$1"; shift
for id in "$@"; do sed -i '' -E "s/^- \[[ x~]\] $id /- [$state] $id /" STATUS.md; done
