package io.github.nixxou.zaxnes;

import org.libsdl.app.SDLActivity;

/**
 * Zaxnes on Android: SDL's activity, which loads the libraries below and runs {@code SDL_main} of
 * the last one ({@code libmain.so}, the crate zax-android).
 */
public class ZaxnesActivity extends SDLActivity {
    @Override
    protected String[] getLibraries() {
        // SDL (its hidapi, which SDL's Java controller code calls, is built into it), then the
        // app's library, whose SDL_main SDL runs.
        return new String[] { "SDL2", "main" };
    }
}
