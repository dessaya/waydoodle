= Waydoodle

This is a Wayland application implemented in Rust that allows the user to draw
on the screen (e.g. for presentations).

= Global shortcut

The application listens for the SIGUSR1 signal to toggle the drawing mode on and
off. You can register a global shortcut on your desktop environment that
executes the following command:

```
pkill -SIGUSR1 waydoodle
```
