# GMod Record Press

A desktop app for creating **Working Record Player** album addons for Garry's Mod.

Drop in cover art and tracks. The app writes the Lua, materials, VTF textures, and sound folder that the record player expects — no VTFEdit and no hand-edited autorun files.

## What it produces

```
recordplayer_<id>/
  addon.json
  lua/autorun/recordplayer-<id>.lua
  materials/recordplayer/<id>/cover.png
  materials/recordplayer/<id>/case_front.vtf + .vmt
  materials/recordplayer/<id>/case_back.vtf + .vmt
  materials/recordplayer/<id>/vinyl.vtf + .vmt
  sound/recordplayer/<id>/*.mp3
```

Optional extras next to that folder:

- `.gma` for [gmpublisher](https://github.com/WilliamVenner/gmpublisher)
- `512×512` workshop JPEG

## Develop

Needs Rust, [Bun](https://bun.sh), and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```bash
bun install
bun run tauri dev
```

Release build:

```bash
bun run tauri build
```

## Use the addon in GMod

1. Export into `garrysmod/addons` (the app can detect this folder).
2. Subscribe to / install [Working Record Player](https://steamcommunity.com/sharedfiles/filedetails/?id=3777821069).
3. Start the game, run `spawnmenu_reload` if the album is missing.
4. Spawn the record player and the album from the entities tab.

This tool does not upload to the Steam Workshop. Use gmpublisher on the exported folder or `.gma`.

## Tests

```bash
cd src-tauri
cargo test
```
