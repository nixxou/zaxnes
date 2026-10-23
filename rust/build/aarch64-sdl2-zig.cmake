# CMake toolchain file for the SDL2 link library when cross-building with cargo-zigbuild
# (SDL2_TOOLCHAIN). It includes the toolchain file that cargo-zigbuild generates (zig cc for the
# requested glibc baseline) and turns off libsamplerate, whose host library cannot be linked
# into an aarch64 library (SDL2 resamples with its own code).
if(DEFINED ENV{CMAKE_TOOLCHAIN_FILE_aarch64_unknown_linux_gnu})
    include("$ENV{CMAKE_TOOLCHAIN_FILE_aarch64_unknown_linux_gnu}")
else()
    message(FATAL_ERROR "run this through cargo zigbuild")
endif()
set(SDL_LIBSAMPLERATE OFF CACHE BOOL "" FORCE)
# This library is only linked against and never shipped: the exported API is the same whatever
# backends are compiled in, so every optional backend that would pick up host (x86_64) headers
# and libraries through pkg-config is turned off.
foreach(opt ALSA PULSEAUDIO JACK PIPEWIRE SNDIO ESD ARTS NAS OSS DBUS IBUS FCITX LIBUDEV
        WAYLAND X11 KMSDRM LIBDECOR HIDAPI VIDEO_VULKAN OPENGL OPENGLES RPATH)
    set(SDL_${opt} OFF CACHE BOOL "" FORCE)
endforeach()
