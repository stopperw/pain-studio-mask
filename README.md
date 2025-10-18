# Pain Studio Mask

**PSM fixes graphics tablet support in Wine by emulating the Wintab library.**
Made for use with **CLIP STUDIO PAINT**, but it will probably work with other apps
that use Wintab (like SAI).

If you want, it can also be used in Windows, for example to make Wintab-only apps
compatible with OpenTabletDriver devices.

# Getting started

*There is a CSP on Wine install guide available [here](https://github.com/stopperw/pain-studio-mask/blob/main/docs/Install_CSP_on_Wine.md)!*

## Installing the emulator

Download the [latest release](https://github.com/stopperw/pain-studio-mask/releases/latest).
Use `wintab32.dll` for x64 apps *(most apps, CSP included)* and `wintab32_x86.dll` ***(you must rename it to `wintab32.dll`)*** for x86 apps.

Put your `wintab32.dll` besides the executable file for your drawing app (for CSP, it should be at `C:\Program Files\CELSYS\CLIP STUDIO 1.5\CLIP STUDIO PAINT\wintab32.dll`)

In `winecfg`, add an override for `wintab32` with mode `Native then Builtin` (or `Native (Windows)` to make sure the app is using PSM)

## Installing the client

Download and setup OpenTabletDriver, if you haven't already -
[Linux](https://opentabletdriver.net/Wiki/Install/Linux) |
[Windows](https://opentabletdriver.net/Wiki/Install/Windows) |
[macOS](https://opentabletdriver.net/Wiki/Install/MacOS)

Download the latest OTD plugin from [psm-otd releases](https://github.com/stopperw/psm-otd/releases/latest)
and put in into `~/.config/OpenTabletDriver/Plugins/PSM/psm-otd.dll` (or your platform equivalent)
(create the `PSM` folder if it doesn't exist).

Launch OTD, enable the client in `Filters` tab, press Apply and interact with the tablet.

Copy `psm.json` from the plugin folder (`~/.config/OpenTabletDriver/PSM/psm.json`)
to drawing app's folder (for CSP, it should be at `C:\Program Files\CELSYS\CLIP STUDIO 1.5\CLIP STUDIO PAINT\psm.json`)

## Run your app

If you did everything correctly, the tablet should just work in the app!

If you run your app through console, you should see a `[*] PSM is loaded!` message.

If nothing works, check the Troubleshooting section below.

# Troubleshooting

## App crashes on start

- Verify that you put the `psm.json` besides `wintab32.dll`.
- Try running the app through console to get some debug output.
  You should see some relevant error messages.
- Try running with `RUST_LOG=debug` env variable.

## No PSM log messages

- \[Wine\] Verify that you added the library override in `winecfg`.
- \[Wine\] `wintab32` should be used as the override library name,
  ***not `wintab32.dll`***.
- \[Wine\] Try running with `WINEDLLOVERRIDES="wintab32=n"`
  to ensure that PSM's Wintab32 is used.
- Verify that you put the library into the correct folder, next to
  the app's executable file.
- Verify that you used the DLL for the correct architecture.
  For CSP, you should use the x64 version (`wintab32.dll` in releases).

## Failed to bind! PSM WILL NOT WORK. (AddrInUse)

You have another app that already uses PSM or the port 40302.

If you are using Wine, you can use `wineserver -k` to kill all Wine apps.

On Linux, you can use `sudo lsof -i -P -n | grep 40302` to see what process
is using the port (second column is the PID).

## Config-related errors

Just try copying the generated `psm.json` again. Make sure it is
in the same directory as your `wintab32.dll`.

If it still doesn't work, try copying the config to the directory you are currently in.

The config search order is as follows:

1. Working directory
2. Application executable directory
3. [Config directory](https://docs.rs/dirs/latest/dirs/fn.config_local_dir.html)

# Development

## Prerequisites

> (you can swap the GNU/MSVC toolchain for any other you want,
> if you do, don't forget to swap it in .cargo/config.toml too)

## On Linux (cross-compiling for Windows)

- `rustup target add x86_64-pc-windows-gnu`
- `rustup target add i686-pc-windows-gnu`
- Install the GNU toolchains for Windows
- For x86_64, run `cargo build`, the DLL will be in `target/x86_64-pc-windows-gnu/debug/wintab32.dll`
- For i686, run `cargo build --target i686-pc-windows-gnu`, the DLL will be in `target/i686-pc-windows-gnu/debug/wintab32.dll`
- You can symlink the DLL to your app's folder to make development easier

## On Windows

- Set target in `.cargo/config.toml` to `x86_64-pc-windows-msvc`
- `rustup target add x86_64-pc-windows-msvc`
- `rustup target add i686-pc-windows-msvc`
- Install the MSVC toolchains (if you haven't already)
- For x86_64, run `cargo build`, the DLL will be in `target/(x86_64-pc-windows-msvc/)debug/wintab32.dll`
- For i686, run `cargo build --target i686-pc-windows-msvc`, the DLL will be in `target/i686-pc-windows-msvc/debug/wintab32.dll`

## Tips

- Run with `RUST_LOG=debug` to get more logging
- \[Wine\] Run with `WINEDLLOVERRIDES="wintab32=n"` to make sure that the app uses the emulated wintab32
- `cargo build --target x86_64-pc-windows-gnu --manifest-path <path to this repo>/Cargo.toml --package wintab32`
- [Official Wintab Docs](https://developer-docs.wacom.com/docs/icbt/windows/wintab/wintab-reference) |
  [Wintab Projects](https://docs.thesevenpens.com/drawtab/developers/wintab-api) |
  [Wintab Debug Tools](https://developer-support.wacom.com/hc/en-us/articles/9354461019927-Wintab-diagnostic)

Make a release build with `cargo build --profile release-optimized`.

# FAQ

## What if I don't want to use OpenTabletDriver?

I'm planning to make a Libinput (Wayland) client, but for now, only OTD is supported.

