package io.github.nixxou.zaxnes;

import android.content.pm.ActivityInfo;
import android.view.KeyEvent;

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

    /** The orientation the application asks for (its native code, through SDL's channel). */
    private static final int COMMAND_ORIENTATION = COMMAND_USER + 1;

    /**
     * The application's own messages: the orientation of the screen, 0 landscape, 1 portrait,
     * 2 either as the device is held (owner request). SDL's own orientation hint cannot be
     * changed once its full-screen window exists.
     */
    @Override
    protected boolean onUnhandledMessage(int command, Object param) {
        if (command == COMMAND_ORIENTATION && param instanceof Integer) {
            switch ((Integer) param) {
                case 1:
                    setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR_PORTRAIT);
                    break;
                case 2:
                    setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_FULL_USER);
                    break;
                default:
                    setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE);
                    break;
            }
            return true;
        }
        return super.onUnhandledMessage(command, param);
    }

    /**
     * Back, whatever device sends it, reaches the application as the keyboard's Back (the
     * in-game menu). A handheld's controller (Retroid Pocket) sends it as a gamepad key, which
     * SDL would otherwise take for the pad's Select.
     */
    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.getKeyCode() == KeyEvent.KEYCODE_BACK) {
            if (event.getAction() == KeyEvent.ACTION_DOWN && event.getRepeatCount() == 0) {
                SDLActivity.onNativeKeyDown(KeyEvent.KEYCODE_BACK);
            } else if (event.getAction() == KeyEvent.ACTION_UP) {
                SDLActivity.onNativeKeyUp(KeyEvent.KEYCODE_BACK);
            }
            return true;
        }
        return super.dispatchKeyEvent(event);
    }
}
