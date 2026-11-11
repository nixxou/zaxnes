"""Builds the Android APK of Zaxnes (arm64-v8a), without Gradle.

Usage: python -I build/package-android.py

Needs the Android SDK (ANDROID_HOME: platforms;android-34, build-tools;35.0.0 (34's d8 fails on SDL's
Java), cmake;3.22.1 and an
NDK r27, ANDROID_NDK_HOME), a JDK (JAVA_HOME, javac and keytool), the Rust target
aarch64-linux-android and cargo-ndk.

1. Builds the crate zax-android (libmain.so) and SDL2 from source as libSDL2.so (cargo ndk; SDL's
   CMake through Ninja, its C files with build/android/sdl-ndk27.h).
2. Compiles SDL's Java activity (the bundled SDL's own sources) and build/android/java against
   android.jar, then into classes.dex (d8).
3. Links build/android/AndroidManifest.xml (aapt2), adds classes.dex and the two libraries,
   aligns the APK (zipalign) and signs it (apksigner) with the key of ZAXNES_ANDROID_KEYSTORE
   (password in ZAXNES_ANDROID_KEYSTORE_PASSWORD), or a debug key made once in the user's
   folder: the key never goes into the repository.

The APK goes to target-android/package/Zaxnes-<version>-android-arm64.apk.
"""

import glob
import os
import shutil
import subprocess
import sys
import zipfile

RUST = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ANDROID = os.path.join(RUST, 'build', 'android')
TARGET = os.path.join(RUST, 'target-android')
ABI = 'arm64-v8a'
TRIPLE = 'aarch64-linux-android'
MIN_SDK = 24
TARGET_SDK = 34
BUILD_TOOLS = '35.0.0'


def need(name):
    value = os.environ.get(name)
    if not value or not os.path.isdir(value):
        sys.exit(f'{name}: not set to a folder')
    return value


def run(command, **kwargs):
    print('>', ' '.join(str(part) for part in command[:4]), '...', flush=True)
    subprocess.run(command, check=True, **kwargs)


def tool(folder, name):
    for extension in ('.exe', '.bat', ''):
        path = os.path.join(folder, name + extension)
        if os.path.isfile(path):
            return path
    sys.exit(f'{name}: not found in {folder}')


def version():
    for line in open(os.path.join(RUST, 'Cargo.toml'), encoding='utf-8'):
        if line.startswith('version = '):
            return line.split('"')[1]
    sys.exit('no version in Cargo.toml')


def version_code(text):
    major, minor, patch = (int(part) for part in text.split('.'))
    return major * 10000 + minor * 100 + patch


def build_libraries(sdk, ndk):
    cmake = os.path.join(sdk, 'cmake', '3.22.1', 'bin')
    env = dict(os.environ)
    env.update({
        'ANDROID_NDK': ndk,
        'CMAKE_GENERATOR': 'Ninja',
        'CMAKE_MAKE_PROGRAM': os.path.join(cmake, 'ninja.exe' if os.name == 'nt' else 'ninja'),
        'PATH': cmake + os.pathsep + env['PATH'],
        'CARGO_TARGET_DIR': TARGET,
        'CFLAGS_aarch64_linux_android':
            '-include ' + os.path.join(ANDROID, 'sdl-ndk27.h').replace('\\', '/'),
    })
    run(['cargo', 'ndk', '-t', ABI, '-P', str(MIN_SDK), 'build', '--release', '-p',
         'zax-android'], cwd=RUST, env=env)
    out = os.path.join(TARGET, TRIPLE, 'release')
    # The C++ runtime of the slang shaders (glslang, through librashader): libmain.so needs it
    # at start-up, and the NDK's shared one is the one Android's loader finds in the APK.
    cxx = glob.glob(os.path.join(ndk, 'toolchains', 'llvm', 'prebuilt', '*', 'sysroot', 'usr',
                                 'lib', TRIPLE, 'libc++_shared.so'))
    if not cxx:
        sys.exit('libc++_shared.so is not in the NDK')
    return [os.path.join(out, name) for name in ('libSDL2.so', 'libmain.so')] + cxx[:1]


def sdl_java():
    """The Java sources of the SDL that sdl2-sys builds (the same version as libSDL2.so)."""
    home = os.environ.get('CARGO_HOME', os.path.join(os.path.expanduser('~'), '.cargo'))
    found = glob.glob(os.path.join(home, 'registry', 'src', '*', 'sdl2-sys-0.38.0', 'SDL',
                                   'android-project', 'app', 'src', 'main', 'java'))
    if not found:
        sys.exit('the sources of sdl2-sys 0.38.0 are not in the cargo registry')
    return found[0]


def compile_java(sdk, work):
    android_jar = os.path.join(sdk, 'platforms', f'android-{TARGET_SDK}', 'android.jar')
    sources = []
    for root in (sdl_java(), os.path.join(ANDROID, 'java')):
        for folder, _, files in os.walk(root):
            sources += [os.path.join(folder, name) for name in files if name.endswith('.java')]
    classes = os.path.join(work, 'classes')
    os.makedirs(classes)
    javac = os.path.join(os.environ['JAVA_HOME'], 'bin', 'javac')
    run([javac, '-nowarn', '-source', '8', '-target', '8', '-bootclasspath', android_jar, '-d',
         classes] + sources)
    class_files = glob.glob(os.path.join(classes, '**', '*.class'), recursive=True)
    dex = os.path.join(work, 'dex')
    os.makedirs(dex)
    tools = os.path.join(sdk, 'build-tools', BUILD_TOOLS)
    run([tool(tools, 'd8'), '--release', '--min-api', str(MIN_SDK), '--lib', android_jar,
         '--output', dex] + class_files)
    return os.path.join(dex, 'classes.dex')


def keystore(work):
    path = os.environ.get('ZAXNES_ANDROID_KEYSTORE')
    if path:
        return path, os.environ.get('ZAXNES_ANDROID_KEYSTORE_PASSWORD', '')
    path = os.path.join(os.path.expanduser('~'), '.android', 'zaxnes-debug.keystore')
    if not os.path.isfile(path):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        keytool = os.path.join(os.environ['JAVA_HOME'], 'bin', 'keytool')
        run([keytool, '-genkeypair', '-keystore', path, '-storepass', 'android', '-keypass',
             'android', '-alias', 'zaxnes', '-keyalg', 'RSA', '-keysize', '2048', '-validity',
             '10000', '-dname', 'CN=Zaxnes debug'])
    return path, 'android'


def main():
    sdk = need('ANDROID_HOME')
    ndk = need('ANDROID_NDK_HOME')
    need('JAVA_HOME')
    name = version()
    libraries = build_libraries(sdk, ndk)

    work = os.path.join(TARGET, 'apk')
    os.makedirs(work, exist_ok=True)
    for entry in os.listdir(work):
        path = os.path.join(work, entry)
        if os.path.isdir(path):
            shutil.rmtree(path)
        else:
            os.remove(path)
    dex = compile_java(sdk, work)

    tools = os.path.join(sdk, 'build-tools', BUILD_TOOLS)
    android_jar = os.path.join(sdk, 'platforms', f'android-{TARGET_SDK}', 'android.jar')
    linked = os.path.join(work, 'linked.apk')
    run([tool(tools, 'aapt2'), 'link', '-o', linked, '--manifest',
         os.path.join(ANDROID, 'AndroidManifest.xml'), '-I', android_jar, '--min-sdk-version',
         str(MIN_SDK), '--target-sdk-version', str(TARGET_SDK), '--version-code',
         str(version_code(name)), '--version-name', name])
    with zipfile.ZipFile(linked, 'a', zipfile.ZIP_DEFLATED) as apk:
        apk.write(dex, 'classes.dex')
        for library in libraries:
            apk.write(library, f'lib/{ABI}/{os.path.basename(library)}')

    package = os.path.join(TARGET, 'package')
    os.makedirs(package, exist_ok=True)
    aligned = os.path.join(work, 'aligned.apk')
    run([tool(tools, 'zipalign'), '-f', '-p', '4', linked, aligned])
    out = os.path.join(package, f'Zaxnes-{name}-android-arm64.apk')
    store, password = keystore(work)
    run([tool(tools, 'apksigner'), 'sign', '--ks', store, '--ks-pass', f'pass:{password}',
         '--out', out, aligned])
    print(f'package: {out}')


if __name__ == '__main__':
    main()
