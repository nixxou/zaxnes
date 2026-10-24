# CMake toolchain file for the SDL2 link library of the Linux x86_64 package (SDL2_TOOLCHAIN),
# the x86_64 twin of aarch64-sdl2-zig.cmake: zig cc for the requested glibc baseline, and every
# optional backend off, since this copy is only linked against and never shipped.
if(DEFINED ENV{CMAKE_TOOLCHAIN_FILE_x86_64_unknown_linux_gnu})
    include("$ENV{CMAKE_TOOLCHAIN_FILE_x86_64_unknown_linux_gnu}")
else()
    message(FATAL_ERROR "run this through cargo zigbuild")
endif()
set(SDL_LIBSAMPLERATE OFF CACHE BOOL "" FORCE)
foreach(opt ALSA PULSEAUDIO JACK PIPEWIRE SNDIO ESD ARTS NAS OSS DBUS IBUS FCITX LIBUDEV
        WAYLAND X11 KMSDRM LIBDECOR HIDAPI VIDEO_VULKAN OPENGL OPENGLES RPATH)
    set(SDL_${opt} OFF CACHE BOOL "" FORCE)
endforeach()
