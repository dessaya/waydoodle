# Waydoodle

A minimalistic Wayland screen annotation tool. Draw on your screen during
presentations, demos, or video calls — on any Wayland compositor.

Unlike other similar tools like [Gromit-MPX](https://github.com/bk138/gromit-mpx)
or [Wayscriber](https://wayscriber.com/), Waydoodle provides only the essentials:

- Tray icon with menu
- Global shortcut (see [below](#global-shortcut))
- Draw with the mouse or tablet
- Change color (<kbd>r</kbd>, <kbd>g</kbd>, <kbd>b</kbd>, <kbd>y</kbd>, <kbd>m</kbd>, <kbd>n</kbd>)
- Erase (<kbd>e</kbd>)
- Clear (<kbd>c</kbd>)
- Black/white background (<kbd>.</kbd>, <kbd>,</kbd>)
- Undo (<kbd>u</kbd>)
- On-screen help (<kbd>F1</kbd>)

## Installation

### From source

Make sure you have a [Rust toolchain](https://rustup.rs/) installed, then:

```
git clone https://github.com/dessaya/waydoodle.git
cd waydoodle
cargo install --path .
```

### Arch Linux (AUR)

Install the [`waydoodle`](https://aur.archlinux.org/packages/waydoodle)
package with your preferred AUR helper:

```
paru -S waydoodle
```

## Usage

Launch Waydoodle from your application menu or from a terminal:

```
waydoodle
```

A tray icon will appear. Use its menu, or send `SIGUSR1` to toggle the
annotation overlay on and off:

```
pkill -SIGUSR1 waydoodle
```

If the XDG Layer Shell protocol is supported by your compositor, the overlay
will be displayed on top of all windows and will not receive input events when
inactive.

Hit <kbd>Esc</kbd> or send `SIGUSR2` to close the overlay (destroying the
current drawing):

```
pkill -SIGUSR2 waydoodle
```

While the overlay is focused:

| Key | Action |
|-----|--------|
| <kbd>r</kbd> | Red pen |
| <kbd>g</kbd> | Green pen |
| <kbd>b</kbd> | Blue pen |
| <kbd>y</kbd> | Yellow pen |
| <kbd>m</kbd> | Magenta pen |
| <kbd>n</kbd> | Cyan pen |
| <kbd>e</kbd> | Eraser |
| <kbd>c</kbd> | Clear all |
| <kbd>.</kbd> | Black background |
| <kbd>,</kbd> | White background |
| <kbd>u</kbd> | Undo |
| <kbd>F1</kbd> | Toggle help screen |
| <kbd>Esc</kbd> | Close overlay |

## Global shortcuts

The XDG Global Shortcuts protocol is not yet widely supported by Wayland
compositors, so Waydoodle falls back to listening for the `SIGUSR1` and
`SIGUSR2` signals to toggle the overlay and close it, respectively.

Register a global shortcut in your desktop environment or window
manager of choice that executes:

```
pkill -SIGUSR1 waydoodle
```

```
pkill -SIGUSR2 waydoodle
```

## Acknowledgements

Waydoodle uses the [Tamzen](https://github.com/sunaku/tamzen-font) bitmap font
for on-screen text rendering.

## License

MIT. See [LICENSE](LICENSE) for details.
