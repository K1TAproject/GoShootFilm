param(
    [string]$Gradle = "D:\SPSir\.tools\gradle-8.13\bin\gradle.bat"
)

$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
$local = Join-Path $root ".local"
$signing = Join-Path $local "release-signing.properties"
if (-not (Test-Path -LiteralPath $signing)) {
    throw "Missing $signing. Create a GSFMobile release key first; do not reuse the SPSir key."
}

$javaFile = Join-Path $local "java-home.txt"
if (Test-Path -LiteralPath $javaFile) {
    $env:JAVA_HOME = (Get-Content -LiteralPath $javaFile -Raw).Trim()
}
if (-not $env:JAVA_HOME -or -not (Test-Path -LiteralPath "$env:JAVA_HOME\bin\jlink.exe")) {
    throw "Set JAVA_HOME to a full JDK 17+ (must contain bin\jlink.exe), or write its path to .local/java-home.txt."
}

$sdkFile = Join-Path $local "android-home.txt"
if (Test-Path -LiteralPath $sdkFile) {
    $env:ANDROID_HOME = (Get-Content -LiteralPath $sdkFile -Raw).Trim()
}
if (-not $env:ANDROID_HOME -or -not (Test-Path -LiteralPath "$env:ANDROID_HOME\platforms\android-35")) {
    throw "Set ANDROID_HOME to an Android SDK with platform 35, or write its path to .local/android-home.txt."
}
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:GRADLE_USER_HOME = Join-Path $root ".gradle-home"

if (-not (Test-Path -LiteralPath $Gradle)) {
    $Gradle = Join-Path $root "gradlew.bat"
}

& $Gradle -p $root :app:assembleRelease "-Pkotlin.compiler.execution.strategy=in-process" --console=plain
if ($LASTEXITCODE -ne 0) { throw "Android release build failed." }

$output = Join-Path $root "app\build\outputs\apk\release"
$metadata = Get-Content -LiteralPath (Join-Path $output "output-metadata.json") -Raw | ConvertFrom-Json
$element = $metadata.elements[0]
$version = $element.versionName
$versionCode = [int]$element.versionCode
$apk = Join-Path $output $element.outputFile
if (-not (Test-Path -LiteralPath $apk)) { throw "Release APK was not produced." }

$buildTools = Join-Path $env:ANDROID_HOME "build-tools\35.0.0"
$apksigner = Join-Path $buildTools "apksigner.bat"
if (-not (Test-Path -LiteralPath $apksigner)) { throw "apksigner was not found under build-tools\35.0.0." }
& $apksigner verify $apk
if ($LASTEXITCODE -ne 0) { throw "APK signature verification failed." }

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force $dist | Out-Null
$name = "GSFMobile-$version.apk"
$destination = Join-Path $dist $name
Copy-Item -LiteralPath $apk -Destination $destination -Force
$hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
$update = [ordered]@{
    applicationId = $metadata.applicationId
    appName = "GSFMobile"
    versionCode = $versionCode
    versionName = $version
    minSdk = [int]$metadata.minSdkVersionForDexing
    apkName = $name
    sha256 = $hash
    releaseTag = "v$version"
}
[IO.File]::WriteAllText(
    (Join-Path $dist "mobile-update.json"),
    ($update | ConvertTo-Json),
    (New-Object Text.UTF8Encoding($false))
)
Write-Host "Release APK: $destination"
Write-Host "Updater manifest: $dist\mobile-update.json"
Write-Host "SHA-256: $hash"
Write-Host "Upload both files to the same GitHub Release as the desktop installer."
