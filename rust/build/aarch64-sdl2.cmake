# CMake toolchain file for the bundled SDL2 when cross-building for Linux aarch64 (SDL2_TOOLCHAIN).
#
# The compilers come from CC/CXX_aarch64_unknown_linux_gnu. SDL2's KMSDRM, GBM, EGL and GLES
# backends are loaded with dlopen on the device: the build only needs their headers, which are
# architecture independent and are taken from the host (libdrm-dev, libgbm-dev, libegl-dev,
# libgles-dev). libsamplerate has a host header but no aarch64 library to link, and SDL2 would
# then reference it unresolved: it is turned off (SDL2 resamples with its own code).
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_SYSTEM_PROCESSOR aarch64)
set(SDL_LIBSAMPLERATE OFF CACHE BOOL "" FORCE)
