# Vendored dependencies

`bevy_winit` is copied from the published Bevy 0.19.1 crate. Its source is
unchanged; only its Cargo feature definitions differ:

- `wayland` enables the native Wayland backend without choosing a decoration
  renderer;
- `wayland-csd-adwaita` preserves Bevy's original titled decorations;
- `wayland-csd-adwaita-notitle` selects Winit's title-free decorations and
  avoids the `ab_glyph` → `ttf-parser` dependency path.

IDApTIK selects the title-free variant while retaining X11 support through
Bevy's normal platform defaults. The upstream MIT and Apache-2.0 license files
are preserved beside the vendored source.

Remove this patch after a released Bevy version exposes independent Wayland CSD
features and IDApTIK has upgraded to it.
