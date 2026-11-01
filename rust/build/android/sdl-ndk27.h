/* Included before every C file of SDL 2.26.4 (the version sdl2-sys bundles) when it is built for
 * Android with NDK r27: that NDK removed ALooper_pollAll, which SDL's sensor code calls. The NDK's
 * declaration is read first, then the call goes to ALooper_pollOnce, as later SDL releases do (the
 * call polls once without waiting, so the two behave the same there). The flags reach every C
 * build of the crates, assembly files included: those get nothing. */
#if !defined(__ASSEMBLER__) && defined(__ANDROID__)
#include <android/looper.h>
#define ALooper_pollAll ALooper_pollOnce
#endif
