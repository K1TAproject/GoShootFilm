# Tauri + Vue + TypeScript

This template should help get you started developing with Vue 3 and TypeScript in Vite. The template uses Vue 3 `<script setup>` SFCs, check out the [script setup docs](https://v3.vuejs.org/api/sfc-script-setup.html#sfc-script-setup) to learn more.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Release signing

Updater releases are built by `.github/workflows/release.yml`. Configure these GitHub Actions secrets before running the workflow:

- `TAURI_SIGNING_PRIVATE_KEY`: the complete Tauri updater private key.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the private-key password.

The updater reads `latest.json` from the matching GitHub Release. Never commit the private key, its password, or release tokens.
