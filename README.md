# Fugo Launcher

Fugo is a desktop launcher for **Minecraft: Java Edition** and **Horizon, the Rust Edition** of
Minecraft. One window installs, updates and starts both: sign in with a Microsoft account or play
offline, make instances with the loader you want, add mods and modpacks from Modrinth, and keep
Horizon up to date without thinking about it.

It is built with [Tauri 2](https://tauri.app) (a Rust backend and the system web view) and
[React](https://react.dev) with TypeScript and Tailwind CSS on the front end. The window has no
system frame; Fugo draws its own title bar.

---

## Contents

- [Features](#features)
- [Getting started](#getting-started)
- [First launch and accounts](#first-launch-and-accounts)
- [Playing](#playing)
- [Instances](#instances)
- [Mods, modpacks and resource packs](#mods-modpacks-and-resource-packs)
- [Horizon Rust Edition](#horizon-rust-edition)
- [Skins and capes](#skins-and-capes)
- [Settings](#settings)
- [Where Fugo keeps its data](#where-fugo-keeps-its-data)
- [Project layout](#project-layout)
- [Development](#development)
- [Building a release](#building-a-release)
- [Security notes](#security-notes)
- [Known limitations](#known-limitations)
- [Credits](#credits)

---

## Features

- **Two editions, one launcher.** Java instances run Mojang's game. Rust Edition instances run
  Horizon, a clean-room Rust reimplementation of Minecraft.
- **Microsoft sign-in.** A device-code login in your browser, using the same account chain Minecraft
  itself uses. Accounts that own Minecraft: Java Edition play online with their real profile.
- **Offline accounts.** Pick any name (3 to 16 letters, digits or underscores) and a starter skin.
  Works in single player and on offline-mode servers.
- **Several accounts.** Switch between them from the top bar, add or remove them at any time.
- **Instances.** Each one has its own game folder, memory, Java arguments and version. Duplicate,
  import and delete them from the Instances page.
- **Loaders.** Vanilla, Fabric, Quilt, Forge and NeoForge, with the loader's version chosen when you
  create the instance.
- **Mods, modpacks and resource packs** from Modrinth, with search, dependency-aware installs and
  per-file enable or disable.
- **Automatic Horizon updates.** The Rust Edition follows Horizon's Nightly build and downloads and
  prepares new builds in the background.
- **Skins and capes.** Starter skins, skins from a player name, your own skin file, and capes.
- **Clean first run.** A short welcome explains the two ways to play and sets up your first account.

---

## Getting started

### Requirements

- [Node.js](https://nodejs.org) 20 or newer and npm (the project is also set up for pnpm; its
  `pnpm-lock.yaml` is kept current).
- [Rust](https://rustup.rs) stable, 1.85 or newer (the crate uses the 2024 edition).
- The Tauri system dependencies for your platform. See the
  [Tauri prerequisites](https://tauri.app/start/prerequisites/). On Linux that means WebKitGTK and
  the usual build tools.
- Java is **not** required to install. Fugo downloads the Java runtime each Minecraft version needs.

### Run it in development

```sh
cd launcher
npm install
npm run tauri dev
```

This starts the Vite dev server and the Tauri window, and rebuilds the Rust side when it changes.

### Build the front end only

```sh
npm run build      # type-checks (tsc --noEmit), then bundles to dist/
npm run preview    # serves dist/ locally
```

Opening the front end in a plain browser (with `npm run dev`) works for layout work. Anything that
talks to the launcher's backend (accounts, instances, launching) needs the Tauri window, and shows
empty or disabled controls in a browser.

### Run the Rust tests

```sh
cd src-tauri
cargo test
```

Some tests download large files, so they are marked `#[ignore]`. Run them on purpose with
`cargo test -- --ignored`.

---

## First launch and accounts

Fugo starts with no accounts. The first screen is a short welcome:

1. **Hello.** What Fugo is and what it does.
2. **How do you want to play?** Choose one:
   - **Sign in with Microsoft** (recommended). Fugo shows a short code and opens Microsoft's sign-in
     page in your browser. Type the code, sign in with the account that owns Minecraft: Java
     Edition, and approve. Fugo waits for the approval, then saves the account and makes it the
     active one.
   - **Play offline.** Enter a name and pick a starter skin. The account lives on this computer.
3. **Done.** Enter Fugo.

The welcome comes back whenever there are no accounts, for example after you remove the last one.

### Microsoft accounts

Fugo follows the standard Microsoft sign-in for Minecraft:

1. A device code from Microsoft (the code and page are shown in Fugo; the page also opens in your
   browser).
2. Microsoft account approval, then Xbox Live and XSTS.
3. Minecraft's login service, which checks that the account owns Minecraft: Java Edition.
4. Your Minecraft profile: name, UUID and active skin.

If something stops the sign-in, Fugo says what. Typical cases:

- The account has no Xbox profile. Make one at xbox.com and try again.
- The account is a child account. An adult has to add it to a family first.
- The account does not own Minecraft: Java Edition.
- The account has never opened Minecraft, so it has no profile yet. Open Minecraft once.

A Microsoft account's sign-in is renewed automatically: Fugo refreshes the Minecraft token shortly
before it expires, when you launch. If the renewal fails (for example after a password change),
Fugo asks you to sign in with Microsoft again.

Fugo uses the public client ID that Minecraft's own launchers use (`00000000402b5328`).

### Offline accounts

An offline account has a name you choose. The launcher gives it a stable UUID derived from the
name, the way Minecraft does for offline play. Offline accounts can't join servers that require
Mojang's sign-in; they work in single player and on offline-mode servers.

### Managing accounts

Click the account name in the top bar to open the account menu:

- Click an account to make it active.
- **Add offline account:** type a name and press Enter.
- **Sign in with Microsoft:** opens the same sign-in as the welcome.
- **Remove:** hover an account and click the trash icon, then confirm. If you remove the active
  account, the next account in the list becomes active.

---

## Playing

Choose an instance on the Instances page, or pick one on Home, and press **Play**. Fugo:

1. Installs the version if it is missing (the game, its libraries and assets, from Mojang).
2. Installs the Java runtime that version needs, if it isn't already installed.
3. Starts the game with the active account. Java launches use your Microsoft token, or an
   offline session for an offline account.

The Home page shows install progress, and a running game is shown as playing. Each instance's
log is available from its page.

**After launch** (Settings) decides what the launcher window does once the game starts: close,
minimize, or keep running in the background.

---

## Instances

An instance is a folder with its own saves, mods, options and resource packs, plus the settings
that start it:

- **Edition:** Java or Rust.
- **Game version** and, for Java, a **loader** (Vanilla, Fabric, Quilt, Forge, NeoForge) and the
  loader's version.
- **Memory** (minimum and maximum) and **Java arguments**.
- **Game folder.** New instances get their own folder. Import an existing folder or a `.zip` of an
  instance from another launcher.

The Instances page lists them, and the dashboard shows an instance's mods, resource packs, shaders
and logs. Duplicate an instance to try a change without touching the original.

Instances are stored in `~/.minecraft/death-client/instances`. Fugo shares the vanilla Minecraft
folder (`~/.minecraft`) for versions, libraries and assets, so a version downloaded once is reused
by every instance.

---

## Mods, modpacks and resource packs

The Mods page searches [Modrinth](https://modrinth.com) by project type (mod, modpack, resource pack,
shader) and sort order (follows, relevance, downloads, newest, updated), filtered to the instance's
version and loader.

- **Install a mod or pack** into an instance. Required dependencies are installed with it.
- **Modpacks** create a new instance with the pack's version, loader and files.
- **Enable or disable** a mod or pack without deleting it. Disabled files stay in the folder.
- **Remove** a mod or pack from the instance.

Fugo talks to Modrinth's public API and identifies itself with its own user agent.

---

## Horizon Rust Edition

Horizon is a Rust reimplementation of Minecraft. Fugo installs and runs it:

- **Installation.** Fugo downloads the latest **Nightly** build of Horizon from its GitHub release,
  checks its SHA-256 checksum against the release's `SHA256SUMS`, and unpacks it. Each build goes
  into its own folder, named after its checksum, so a new Nightly never overwrites the one you run.
- **Preparation.** Horizon needs game files made from your own Minecraft install. Fugo installs
  Minecraft 26.3 from Mojang, then runs Horizon's `horizon-prepare` on it, the same steps Horizon's
  documentation describes. This happens once per build.
- **Updates.** Fugo checks for a new Nightly when it starts and every 30 minutes while it's open. A
  new build downloads and prepares in the background. Only players with a Rust Edition instance (or
  an installed Horizon build) get background updates, and only Horizon is updated this way; Java
  Minecraft is never touched.
- **Cleanup.** The two newest Horizon builds are kept. Older ones are deleted, but only when no game
  is running.
- **Your own build.** If you point Settings at your own Horizon binary and prepared resources, Fugo
  uses them as they are and doesn't update Horizon.

Turn background updates off in **Settings → Horizon Rust Edition → Update Horizon automatically**.
With them off, Fugo still installs Horizon the first time you press Play.

---

## Skins and capes

- **Starter skins** are chosen in the welcome and can be changed on the **Customize** page.
- **Skins from a player name** load the skin of any Minecraft player by name.
- **Your own skin** from an image file (a standard 64×64 Minecraft skin, classic or slim arms).
- **Capes** from the built-in list, or a cape from a player name.

For **offline** accounts, skins and capes are uploaded to a skin service before a Java game
starts, and authlib-injector points the game at it, so other players see them on offline-mode
servers. **Microsoft** accounts keep the skin on Mojang's side. Fugo does not upload skins for
Microsoft accounts, because doing so would interfere with online sign-in.

The skin service and the lookups use third-party services (the Aether skin service, Mojang's
textures and the Minecraft profile API). If they are unreachable, Fugo logs a warning and the game
starts without custom skins.

---

## Settings

Open **Settings** from the sidebar. The options:

| Setting | What it does | Default |
|---|---|---|
| Default RAM | Memory for new Java instances | 4096 MB |
| NVIDIA PRIME discrete GPU offload | Sets `__NV_PRIME_RENDER_OFFLOAD` for Horizon | On |
| Performance mode | Turns off blur and transitions in the launcher | Off (on automatically on low-end machines) |
| After launch | Close, minimize or keep the window open once a game starts | Close |
| Update Horizon automatically | Background Nightly checks and preparation | On |
| JVM arguments | Extra flags for every Java run | Sensible defaults |
| Window size | Game window width and height | 854 × 480 |

Settings are saved to `settings.json` as you change them.

---

## Where Fugo keeps its data

| What | Where |
|---|---|
| Launcher settings and accounts | `<data dir>/fugo-launcher/` (`settings.json`, `accounts.json`) |
| Horizon builds | `<data dir>/fugo-launcher/horizon/` |
| Instances | `~/.minecraft/death-client/instances/` |
| Minecraft versions, libraries, assets, Java runtimes | `~/.minecraft/` |

`<data dir>` is the platform's standard application-data folder, from the `dirs` crate:
`~/.local/share` on Linux, `~/Library/Application Support` on macOS, and `%APPDATA%` on Windows.

---

## Project layout

```
launcher/
├── src/                      React front end
│   ├── App.tsx               the main window: pages, top bar, settings, accounts
│   ├── components/
│   │   ├── Welcome.tsx       first-run welcome
│   │   ├── MicrosoftSignIn.tsx  the Microsoft device-code dialog
│   │   ├── AccountSwitcher.tsx  the account menu
│   │   ├── InstanceDashboard.tsx, CustomizeTab.tsx, LoaderPicker.tsx, …
│   ├── model.ts              shared types (Instance, Account)
│   ├── customize.ts          cursor packs, themes, backgrounds
│   └── App.css               styles and animations
├── src-tauri/                Rust backend (Tauri)
│   ├── src/lib.rs            Tauri commands and the app entry point
│   ├── src/accounts.rs       offline and Microsoft accounts on disk
│   ├── src/instance_manager.rs  instances: create, import, duplicate, mods
│   ├── src/launcher_engine.rs   launch dispatch, running processes, logs
│   ├── src/modrinth.rs       Modrinth search and installs
│   ├── src/settings.rs       launcher settings
│   ├── src/models.rs         shared data types
│   └── src/mc/               Minecraft
│       ├── microsoft.rs      Microsoft sign-in, token renewal
│       ├── horizon.rs        Horizon releases, preparation, background updater
│       ├── install.rs        version and library downloads
│       ├── java.rs           Java runtime download
│       ├── launch.rs         Java and Horizon launches
│       ├── args.rs           Java command-line construction
│       ├── loader.rs, forge.rs  Fabric/Quilt and Forge/NeoForge installers
│       └── appearance.rs     skins, capes, authlib-injector
├── public/                   static assets (backgrounds, skins, capes, resources)
├── package.json
└── vite.config.ts
```

---

## Development

### Adding a Tauri command

1. Write the function in a module under `src-tauri/src/`.
2. Mark it `#[tauri::command]` (use `async fn` for anything that touches the network).
3. Register it in `tauri::generate_handler![…]` in `src-tauri/src/lib.rs`.
4. Call it from the front end with `invoke("command_name", { argName })`. Tauri converts
   `snake_case` argument names to `camelCase` in the front end.

### Adding a front-end screen

Screens that replace the whole window (like the welcome) are rendered from `App.tsx` before the main
layout, and must come before any early return. The main layout uses hooks, so put new hooks above
the first early return.

### Tests

- `cargo test` covers the account name rule, Microsoft token handling and profile IDs, the Horizon
  build folders and pruning, settings defaults, and the Java argument builder.
- `npm run build` type-checks the front end. There is no front-end unit test suite yet.

---

## Building a release

```sh
npm run tauri build
```

This produces the platform's installer or bundle under `src-tauri/target/release/bundle/`. The
bundle targets are set in `src-tauri/tauri.conf.json`.

The app's version is kept in three places: `package.json`, `src-tauri/Cargo.toml` and
`src-tauri/tauri.conf.json`. Change all three together.

---

## Security notes

- **Microsoft tokens are stored in plain text** in `accounts.json` in the data folder, the same way
  the older Fugo launcher stored them. Anyone who can read your user folder can read them. Encrypting
  that file is the next thing to do.
- The web view can read the saved accounts, tokens included. Fugo's own UI never shows the tokens.
- Horizon builds are checked against the SHA-256 checksums published with the same release. Horizon
  also signs its releases (`SHA256SUMS.sig`); Fugo does not verify that signature yet.
- Modrinth downloads are installed as published. Only install mods you trust.

---

## Known limitations

- Microsoft sign-in has been checked against Microsoft's device-code endpoint, but a full sign-in
  and launch with a real account is not yet covered by automated tests.
- Java launches with a Microsoft account have not been exercised end to end in every loader.
- Background Horizon updates only run while the launcher is open.
- Skin and cape lookups depend on third-party services.
- The front end has no automated tests.
- The window controls and the frameless title bar were tested in a browser, not in every desktop
  environment.

---

## Credits

- Fugo's device-code sign-in and skin service follow the approach of the Aether Launcher, which
  builds on OfflineMinecraftLauncher.
- Horizon is by the Horizon team (`horizon-rust/horizon`).
- Built with Tauri, React, Vite, Tailwind CSS, lucide icons and three.js.
- Minecraft is a trademark of Mojang Studios and Microsoft. Fugo is not affiliated with either.

## License

Add the license for your release here.
