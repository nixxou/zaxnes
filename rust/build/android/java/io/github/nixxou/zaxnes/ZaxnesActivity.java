package io.github.nixxou.zaxnes;

import android.content.Intent;
import android.content.pm.ActivityInfo;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.OpenableColumns;
import android.util.Log;
import android.view.KeyEvent;

import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

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

    /**
     * A frontend (ES-DE, Daijishou, a launcher) starts Zaxnes on a game: its request is written
     * before SDL starts the native code, which reads it (GitHub issue #1).
     */
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        if (savedInstanceState == null) {
            writeLaunchRequest(getIntent());
        }
        super.onCreate(savedInstanceState);
    }

    /** Another game sent while Zaxnes runs: it takes the place of the one playing. */
    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        writeLaunchRequest(intent);
    }

    /** The folder of the user data the native code reads the request from. */
    private File launchFolder() {
        File root = getExternalFilesDir(null);
        return root == null ? null : new File(root, "launch");
    }

    /**
     * The game of {@code intent}, if it names one: its data (ACTION_VIEW, "Open with") or the
     * extra "ROM" some frontends use. A {@code content://} game is copied into the user data
     * under its own name, so that its saves stay with it; a {@code file://} path is used as it
     * is. The request is written to a {@code .part} renamed after, so the native code never reads
     * half of it.
     */
    private void writeLaunchRequest(Intent intent) {
        if (intent == null) {
            return;
        }
        Uri uri = intent.getData();
        String rom = null;
        // "Share to Zaxnes": the file is the stream extra.
        if (uri == null && Intent.ACTION_SEND.equals(intent.getAction())) {
            uri = intent.getParcelableExtra(Intent.EXTRA_STREAM);
        }
        if (uri == null) {
            String extra = intent.getStringExtra("ROM");
            if (extra != null && !extra.isEmpty()) {
                uri = extra.startsWith("content://") ? Uri.parse(extra) : null;
                rom = uri == null ? extra : null;
            }
        }
        File folder = launchFolder();
        if (folder == null || (uri == null && rom == null)) {
            return;
        }
        try {
            if (!folder.isDirectory() && !folder.mkdirs()) {
                return;
            }
            // A 3dSen bundle is not a game: it waits in the import folder, where the console
            // interface imports its profiles.
            if (uri != null && "content".equals(uri.getScheme())
                    && displayName(uri).toLowerCase(java.util.Locale.ROOT).endsWith(".bundle")) {
                File root = getExternalFilesDir(null);
                if (root != null) {
                    copyContent(uri, new File(root, "import"));
                }
                return;
            }
            if (uri != null) {
                if ("file".equals(uri.getScheme())) {
                    rom = uri.getPath();
                } else if ("content".equals(uri.getScheme())) {
                    rom = copyContent(uri, new File(folder, "roms"));
                }
            }
            if (rom == null) {
                return;
            }
            File part = new File(folder, "request.part");
            try (OutputStream out = new FileOutputStream(part)) {
                out.write(("rom=" + rom + "\nreturn=1\n").getBytes(StandardCharsets.UTF_8));
            }
            File request = new File(folder, "request");
            if (!part.renameTo(request)) {
                request.delete();
                part.renameTo(request);
            }
        } catch (Exception error) {
            Log.w("Zaxnes", "launch request: " + error);
        }
    }

    /** The name of the file of {@code uri}, never a path; "game.nes" without one. */
    private String displayName(Uri uri) {
        String name = null;
        try (Cursor cursor = getContentResolver().query(
                uri, new String[] { OpenableColumns.DISPLAY_NAME }, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                name = cursor.getString(0);
            }
        } catch (Exception error) {
            name = null;
        }
        if (name == null || name.isEmpty()) {
            name = uri.getLastPathSegment();
        }
        if (name == null || name.isEmpty()) {
            name = "game.nes";
        }
        return name.replace('/', '_').replace('\\', '_');
    }

    /** Copies the file of {@code uri} into {@code folder} under its own name; its path. */
    private String copyContent(Uri uri, File folder) throws Exception {
        String name = displayName(uri);
        if (!folder.isDirectory() && !folder.mkdirs()) {
            return null;
        }
        File target = new File(folder, name);
        File part = new File(folder, name + ".part");
        try (InputStream in = getContentResolver().openInputStream(uri);
             OutputStream out = new FileOutputStream(part)) {
            if (in == null) {
                return null;
            }
            byte[] buffer = new byte[65536];
            int read;
            while ((read = in.read(buffer)) > 0) {
                out.write(buffer, 0, read);
            }
        }
        if (!part.renameTo(target)) {
            target.delete();
            if (!part.renameTo(target)) {
                return null;
            }
        }
        return target.getAbsolutePath();
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
