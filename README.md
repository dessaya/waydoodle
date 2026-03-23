= Waydoodle

This is a minimalistic application that allows the user to annotate on the
screen (e.g. for presentations) on any Wayland compositor. Unlike other similar
tools like [Gromit-MPX](https://github.com/bk138/gromit-mpx) or
[Wayscriber](https://wayscriber.com/), Waydoodle provides only basic features:

[x] Tray icon with menu
[x] Global shortcut (see below)
[x] Draw with the mouse or tablet
[ ] Change color
[ ] Erase
[ ] Undo

In the future I may consider adding more features like different drawing tools
and showing a toolbar, but for now the current set of features is sufficient for
my needs (annotating on the screen during presentations).

= Global shortcut

The XDG Global Shortcuts protocol is not yet widely supported by Wayland
compositors, so Waydoodle falls back to listening for the SIGUSR1 signal to
toggle the drawing mode on and off.

You can register a global shortcut on your desktop environment or window manager
of choice that executes the following command:

```
pkill -SIGUSR1 waydoodle
```
