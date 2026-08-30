<table width="100%">
  <tr>
    <td align="left" width="120">
      <img src="src-tauri/icons/icon.png" alt="GMod Record Press" width="100" />
    </td>
    <td align="right">
      <h1>GMod Record Press</h1>
      <p>
        <a href="https://github.com/Aspenini/gmod-record-press/releases">
          <img alt="GitHub release" src="https://img.shields.io/github/v/release/Aspenini/gmod-record-press?label=release" />
        </a>
        <a href="https://aur.archlinux.org/packages/gmod-record-press">
          <img alt="AUR version" src="https://img.shields.io/aur/version/gmod-record-press?label=AUR&amp;logo=archlinux&amp;cacheSeconds=3600" />
        </a>
      </p>
    </td>
  </tr>
</table>

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

- `.gma` for local packing or a manual Workshop upload
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

## Publish to the Steam Workshop

The app talks to Steamworks directly (same path as gmpublisher). It does not shell out to `gmpublish.exe`.

1. Start Steam and log into an account that owns Garry's Mod.
2. Fill in the album, then use **Publish to Workshop**.
3. New items are created as **private**. Set visibility in the app or on the Workshop page.
4. Later publishes with the saved Workshop ID update that item instead of creating a duplicate.

Steam must be running. If Steam asks you to accept the Workshop legal agreement, do that before the item can go public.

## Tests

```bash
cd src-tauri
cargo test
```
