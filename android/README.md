# GSFMobile Android

GSFMobile is the native Android read-only client for GoShootFilm.

- Package: `com.k1taproject.goshootfilm`
- Launcher label: `GSFMobile`
- Min SDK: 26
- Target SDK: 35
- Version: aligned with the desktop release (`0.3.9`, versionCode `30009`)
- Sync: LAN only, desktop is the source of truth
- Starred edited originals are pre-cached during sync; other photos are not downloaded on tap
- Originals: only formal edited files (PNG/JPEG/WebP), never RAW/CR2/lab scans/TIFF
- Tapping a photo with no local file shows `该图片无本地缓存`
- Local edited images can be saved to the Android gallery (`Pictures/GSFMobile`) or shared
- Android 8/9 asks for storage permission on first save; Android 10+ uses MediaStore

## Requirements

- JDK 17+ with `bin\jlink.exe` (a full JDK, not a JBR without jlink)
- Android SDK with platform 35 and build-tools 35.0.0
- Gradle 8.13, or the wrapper
- For this machine, `.local/java-home.txt` and `.local/android-home.txt` point at the local installations. Both files are ignored.

## Build a debug APK

```powershell
$env:JAVA_HOME = (Get-Content .local\java-home.txt -Raw).Trim()
$env:ANDROID_HOME = (Get-Content .local\android-home.txt -Raw).Trim()
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:GRADLE_USER_HOME = (Resolve-Path .).Path + "\.gradle-home"

D:\SPSir\.tools\gradle-8.13\bin\gradle.bat :app:assembleDebug `
  "-Pkotlin.compiler.execution.strategy=in-process" --console=plain
```

APK: `app\build\outputs\apk\debug\app-debug.apk`

## Build a signed release

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\build-release.ps1
```

Output:

```text
dist\GSFMobile-0.3.9.apk
dist\mobile-update.json
```

The release key/config live in `.local/` and are ignored:

- `.local/gsfmobile-release.jks`
- `.local/release-signing.properties`

Back both up offline. Never reuse the SPSir signing key, and never commit the keystore or its password.

## Release assets

Upload these to the same GitHub Release as the desktop installer:

```text
GSFMobile-0.3.9.apk
mobile-update.json
```

The mobile updater checks a published release for `mobile-update.json`, compares `versionCode`, and opens the APK download page. It does not install silently.

## Film cover pack

The film cards use the desktop catalog images, optimized into the APK:

```powershell
python tools\make_film_pack.py
```

This writes `app/src/main/assets/film-catalog.csv`, `film-stocks/` and `film-thumbs/` as WebP. Re-run it when the desktop film catalog changes.

## Current test status

- `:app:testDebugUnitTest` passes.
- `:app:assembleDebug` and `:app:assembleRelease` pass.
- Release APK signature verified with `apksigner`.
- Real-device testing is still required before distributing to friends.
