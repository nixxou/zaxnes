#!/bin/bash
# PortMaster launch script for Zaxnes (NES emulator with 3D rendering), aarch64.

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

source "$controlfolder/control.txt"
[ -f "${controlfolder}/mod_${CFW_NAME}.txt" ] && source "${controlfolder}/mod_${CFW_NAME}.txt"
get_controls

GAMEDIR="/$directory/ports/zaxnes"
cd "$GAMEDIR" || exit 1

# Everything the program writes (settings, saves, log) stays inside the port folder.
: > "$GAMEDIR/log.txt" && exec > >(tee "$GAMEDIR/log.txt") 2>&1
echo "Zaxnes launch: $CFW_NAME $DEVICE_NAME $DEVICE_ARCH glibc=$CFW_GLIBC gpu=$GPU_DRIVER ${DISPLAY_WIDTH}x${DISPLAY_HEIGHT}"

export XDG_DATA_HOME="$GAMEDIR"
export SDL_GAMECONTROLLERCONFIG="$sdl_controllerconfig"
export ZAXNES_GLES=1

# The program reads the pad through SDL itself; gptokeyb only provides the Select+Start exit.
$GPTOKEYB "zaxnes.aarch64" -c "$GAMEDIR/zaxnes.gptk" &
pm_platform_helper "$GAMEDIR/zaxnes.aarch64" > /dev/null

./zaxnes.aarch64 --handheld --userdata "$GAMEDIR" --roms "$GAMEDIR/roms"

pm_finish
