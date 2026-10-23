#!/bin/bash
# PortMaster launch script for Zaxnes (NES emulator with 3D rendering), aarch64.

# The port folder is found from this script's own location: the ports folder is not at the
# same place on every firmware (muOS /mnt/mmc/ports or /mnt/sdcard/ports, Knulli
# /userdata/roms/ports, ROCKNIX /storage/roms/ports). PortMaster's $directory is the fallback.
SCRIPTDIR="$(cd "$(dirname "$0")" && pwd)"
GAMEDIR="$SCRIPTDIR/zaxnes"

# The first line of the log, before anything can fail: it shows the launcher ran.
LOG="$GAMEDIR/log.txt"
echo "Zaxnes launcher: $(date) script=$0 GAMEDIR=$GAMEDIR" > "$LOG" 2>/dev/null \
  || { LOG="/tmp/zaxnes-log.txt"; echo "Zaxnes launcher: $(date) cannot write $GAMEDIR/log.txt, logging to $LOG" > "$LOG"; }

XDG_DATA_HOME=${XDG_DATA_HOME:-$HOME/.local/share}

if [ -d "/opt/system/Tools/PortMaster/" ]; then
  controlfolder="/opt/system/Tools/PortMaster"
elif [ -d "/opt/tools/PortMaster/" ]; then
  controlfolder="/opt/tools/PortMaster"
elif [ -d "$XDG_DATA_HOME/PortMaster/" ]; then
  controlfolder="$XDG_DATA_HOME/PortMaster"
else
  controlfolder="/roms/ports/PortMaster"
fi

source "$controlfolder/control.txt" >> "$LOG" 2>&1
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt" >> "$LOG" 2>&1
get_controls >> "$LOG" 2>&1

if [ ! -x "$GAMEDIR/zaxnes.aarch64" ] && [ -x "/$directory/ports/zaxnes/zaxnes.aarch64" ]; then
  GAMEDIR="/$directory/ports/zaxnes"
  echo "Zaxnes launcher: the binary is not next to the script, using GAMEDIR=$GAMEDIR" >> "$LOG"
fi
cd "$GAMEDIR" || { echo "Zaxnes launcher: cannot enter $GAMEDIR" >> "$LOG"; exit 1; }

echo "Zaxnes launch: controlfolder=$controlfolder cfw=$CFW_NAME device=$DEVICE_NAME arch=$DEVICE_ARCH glibc=$CFW_GLIBC gpu=$GPU_DRIVER display=${DISPLAY_WIDTH}x${DISPLAY_HEIGHT}" >> "$LOG"

# Everything the program writes (settings, saves, logs/zaxnes.log) stays inside the port folder.
export XDG_DATA_HOME="$GAMEDIR"
export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
export ZAXNES_GLES=1
# TEMPORARY DIAGNOSTIC: the frame profiler writes the cost of each stage of the frame to the log
# every two seconds (and next to the FPS counter). Remove once the handheld frame rate is known.
export ZAXNES_PROFILE=1

# No gptokeyb: the program reads the pad through SDL and has its own exit (Select+Start held
# one second, or "Exit to launcher" in the in-game menu), which saves the settings, the battery
# RAM and the slot index first. gptokeyb's kill switch fired on the first press of Select+Start
# and killed the process before any of that (SIGKILL cannot be caught); a plain SIGTERM is
# turned into a clean quit by SDL.
pm_platform_helper "$GAMEDIR/zaxnes.aarch64" > /dev/null

# The emulation core needs the full CPU speed: a power-saving governor can leave the H700 at a
# fraction of its 1.5 GHz. The governor is set to "performance" for the run and restored after.
GOVERNOR_FILE="/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"
OLD_GOVERNOR="$(cat "$GOVERNOR_FILE" 2>/dev/null)"
echo "Zaxnes launcher: CPU governor '$OLD_GOVERNOR', $(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq 2>/dev/null) kHz of $(cat /sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq 2>/dev/null) kHz" >> "$LOG"
if [ -n "$OLD_GOVERNOR" ] && [ "$OLD_GOVERNOR" != "performance" ]; then
  echo performance > "$GOVERNOR_FILE" 2>> "$LOG" \
    && echo "Zaxnes launcher: CPU governor set to performance for the run" >> "$LOG"
fi

# Plain redirection (no process substitution, which some firmware shells lack). The program
# also writes its own copy of the log to logs/zaxnes.log in the port folder.
./zaxnes.aarch64 --handheld --userdata "$GAMEDIR" --roms "$GAMEDIR/roms" >> "$LOG" 2>&1
echo "Zaxnes launcher: the program exited with status $?" >> "$LOG"
if [ -n "$OLD_GOVERNOR" ] && [ "$OLD_GOVERNOR" != "performance" ]; then
  echo "$OLD_GOVERNOR" > "$GOVERNOR_FILE" 2>/dev/null
fi

pm_finish
