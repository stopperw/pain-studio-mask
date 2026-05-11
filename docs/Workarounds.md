# Workaround #1 - wintab-proxy

1. Download [wintab-proxy](https://github.com/stopperw/wintab-proxy/releases/latest) for the correct
   architecture, make sure it's named `wintab32.dll`.
2. Put this file next to your application's executable file.
3. Download [PSM](https://github.com/stopperw/pain-studio-mask/releases/latest) for the correct
   architecture, rename it to `wintab32_real.dll`.
4. Put this file next to your application's executable file.
5. Continue with the [Getting started guide](https://github.com/stopperw/pain-studio-mask#installing-the-emulator)

# Workaround #2 - PSM in Windows files

1. Download [PSM](https://github.com/stopperw/pain-studio-mask/releases/latest) for the correct
   architecture, make sure it's named `wintab32.dll`.
2. Go to `$WINEPREFIX/drive_c/windows/system32` (for x64 apps) or `$WINEPREFIX/drive_c/windows/syswow64` (for x86 apps)
3. Remove or rename the existing `wintab32.dll` file in that folder.
4. Copy PSM's `wintab32.dll` into that folder.
5. Continue with the [Getting started guide](https://github.com/stopperw/pain-studio-mask#installing-the-emulator)

# Workaround #3 - wintab-proxy in Windows files

1. Download [wintab-proxy](https://github.com/stopperw/wintab-proxy/releases/latest) for the correct
   architecture, make sure it's named `wintab32.dll`.
2. Download [PSM](https://github.com/stopperw/pain-studio-mask/releases/latest) for the correct
   architecture, rename it to `wintab32_real.dll`.
3. Go to `$WINEPREFIX/drive_c/windows/system32` (for x64 apps) or `$WINEPREFIX/drive_c/windows/syswow64` (for x86 apps)
4. Remove or rename the existing `wintab32.dll` file in that folder.
5. Copy wintab-proxy's `wintab32.dll` into that folder.
6. Put PSM's `wintab32_real.dll` into your application's folder.
7. Continue with the [Getting started guide](https://github.com/stopperw/pain-studio-mask#installing-the-emulator)

