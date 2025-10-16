# Install CLIP STUDIO PAINT on Wine

I haven't tested all the features, but everything I've checked works
*(except graphics tablet support, but that's fixable with [PSM](https://github.com/stopperw/pain-studio-mask?tab=readme-ov-file) \*wink\*)*

# Step 1. Install latest Wine

Just install the most recent version of Wine you can.

- On Arch, use `pacman -Syu wine-staging`
- On Fedora, use `dnf install wine`
- On Debian/Ubuntu, follow [these instructions](https://gitlab.winehq.org/wine/wine/-/wikis/Debian-Ubuntu)

# Step 2. Install winetricks dependencies

Run `winetricks corefonts vcrun2022 dxvk`.

# Step 3. Install external dependencies

- Run `winecfg -v win10`.
- Install [Microsoft Edge](https://www.microsoft.com/en-us/edge/download) (lol) and
  [Microsoft Edge WebView2 Evergreen (Standalone Installer)](https://developer.microsoft.com/en-us/microsoft-edge/webview2).

# Step 4. Setup Windows version for CSP

- Run `winecfg -v win7`.
- Run `winecfg` and add an application override for `CLIPStudioPaint.exe`
  (usually located at `~/.wine/drive_c/Program Files/CELSYS/CLIP\ STUDIO 1.5/CLIP STUDIO PAINT/CLIPStudioPaint.exe`):
  set overriden Windows version to `Windows 8.1`.

# Step 5. Install CLIP STUDIO PAINT

Download [CLIP STUDIO PAINT](https://www.clipstudio.net/en/purchase/trial) from the official website.
Very straightforward to install, just click Next a bunch of times, but the installation will take some time.

# Step 6. Run CLIP STUDIO PAINT

Run either the `CLIPStudio.exe` (`~/.wine/drive_c/Program Files/CELSYS/CLIP\ STUDIO 1.5/CLIP STUDIO/CLIPStudio.exe`) launcher
or `CLIPStudioPaint.exe` (`~/.wine/drive_c/Program Files/CELSYS/CLIP\ STUDIO 1.5/CLIP STUDIO PAINT/CLIPStudioPaint.exe`) directly.

It can take up to 5-10 minutes to launch for me, so you might need to be patient. If you don't need the launcher,
I recommend just running `CLIPStudioPaint.exe` directly to save some time.

You can easily login through the launcher, but if you don't want to register through Edge on Wine,
use [this link](https://accounts.clip-studio.com/app/register).

CSP will complain about the current OS being no longer supported, but you can safely ignore it.

# Step 7. If your graphics tablet doesn't work

If you have no pressure sensitivity, or if your cursor is fully invisible on canvas and nothing works,
you can either

## Try changing some settings

- Enable the `Use mouse mode in tablet driver settings` (File > Preferences > Tablet) in CSP
- Max out the `Tip Pressure Feel` in the Graphics Tablet native driver.

or if that doesn't work, reverse the changes and

## Try using Pain Studio Mask

Follow instructions in the [Pain Studio Mask](https://github.com/stopperw/pain-studio-mask?tab=readme-ov-file)
GitHub repository.

# Tips

- When in doubt, run `wineserver -k` (this will kill all running Wine apps) and try again.
- Try `winetricks renderer=vulkan` if something is wrong with graphics.
- \[Wayland\] Try running with `DISPLAY=` (a.k.a. unset the DISPLAY env var)
  to use the experimental Wayland-native Wine.
- \[Pain Studio Mask\] Try running with `WINEDLLOVERRIDES="wintab32=n"`
  to ensure that CSP uses PSM's Wintab32.

# Info

Guide is based on a [WineHQ Test Result](https://appdb.winehq.org/objectManager.php?sClass=version&iId=42586&iTestingId=116257) by Michel Pereira.
