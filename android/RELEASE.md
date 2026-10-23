# GSFMobile 合并发布清单

桌面端和 Android 端必须发布在**同一个 GitHub Release**，否则 `releases/latest` 可能指向缺少 `latest.json` 的 Android-only Release，破坏桌面 Updater。

## 0.3.3 安装器修复

0.3.0/0.3.1 的桌面 Updater 存在 Vue Proxy 包裹 Tauri `Update` 对象的问题，应用内更新会在下载/安装阶段报：

```text
Cannot read private member from an object whose class did not declare it
```

0.3.3 已改用 `shallowRef`，但从 0.3.0/0.3.1 **不能自动更新**到 0.3.3；必须先手动运行 0.3.3 安装包覆盖安装。完成一次手动安装后，0.3.3 及后续版本的应用内更新才可用。

## 1. 升版本

同时修改：

- `package.json` / `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
- `README.md`
- `android/app/build.gradle.kts` 的 `versionName` 和 `versionCode`

沿用当前 scheme：`0.3.0 -> 30000`，`0.3.1 -> 30001`，`0.3.3 -> 30003`，`0.3.4 -> 30004`，`0.3.5 -> 30005`，`0.3.6 -> 30006`，`0.3.7 -> 30007`。

## 2. 验证

```powershell
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml

cd android
.\gradlew.bat :app:testDebugUnitTest :app:assembleRelease
```

## 3. 构建两个安装包

桌面端（带 Updater 签名）：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content "$env:USERPROFILE\.tauri\goshootfilm.key" -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = (Get-Content "$env:USERPROFILE\.tauri\goshootfilm.key.password" -Raw).Trim()
npm run tauri build
```

Android：

```powershell
npm run build:android
```

产物：

```text
src-tauri\target\release\bundle\nsis\GoShootFilm_0.3.7_x64-setup.exe
src-tauri\target\release\bundle\nsis\GoShootFilm_0.3.7_x64-setup.exe.sig
src-tauri\target\release\bundle\nsis\latest.json

android\dist\GSFMobile-0.3.7.apk
android\dist\mobile-update.json
```

## 4. 创建 Release

1. 推送代码，再创建并推送与桌面版本一致的 tag，例如 `v0.3.7`。
2. 等待桌面 Release 草稿生成。
3. 把 Android APK 和 `mobile-update.json` 上传到同一个草稿。
4. 核对：
   - `latest.json` 的版本和安装包 URL 正确；
   - `mobile-update.json` 的 `versionCode`、`sha256`、`apkName` 与 APK 一致；
   - APK 已用 `apksigner verify` 验证；
   - Release 不是 Pre-release，且包含两个安装包。
5. 点击 Publish release。

## 5. 发布后验证

- 桌面端设置中检查更新能发现新版本。
- 手机端设置中检查更新能发现新 `versionCode`，打开 APK 下载页。
- 用旧 APK 覆盖安装新 APK，确认签名一致。
- 不要删除或改名已发布 tag/asset；旧版本可能依赖它们。

## 6. 签名

- Android 使用独立的 `.local/gsfmobile-release.jks`，不要复用 SPSir 的签名。
- 备份 keystore 和密码；丢失后朋友只能卸载重装。
- 桌面 Updater 的 minisign 密钥与 Android APK 签名是两套密钥，互不替代。
