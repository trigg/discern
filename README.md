# Discern

Discern is, like its predecessors [DiscordOverlayLinux](https://github.com/trigg/DiscordOverlayLinux) and [Discover-overlay](https://github.com/trigg/Discover), a Discord overlay for linux.

This one is written in Rust as a project to acclimatise myself to using rust for real projects but so far is a lesson in how to self induce headaches.

While previous projects gave a plethora of user options and tweaks this one aims for a 'one-size-fits-all' in each module.

## Current targets


| Target | Binary | Description |
| ------ | ------ | ----------- |
|        | discern | Generic binary that will read in ENV variables and make a best-guess at what the user will want. Assumes graphical views only, no plans to best-guess terminal versions |
|  x11    | discern-x11     | GTK on x11. Uses combination of highest layer, undecorated window, xshape and xinputshape to draw over top of desktop. |
| wlroots | discern-wlroots | GTK on wayland. Uses wlroots LayerShell to draw over top of desktop |
|  rpc | discern-rpc | terminal application to poll or alter discord state |
| statefile | discern-statefile | terminal or daemon application to dump current state to a file or pipe. |
| gamescope | discern-gamescope | Cairo on XCB. Uses X11 XAtom to mark as overlay window for use in gamescope |
| clispam | discern-clispam | terminal application to output all communication raw to terminal. Useful for debugging |

By default all targets are compiled at once.

To choose a specific target to compile:
```
cargo clean 
cargo build --features "wlroots" --no-default-features
```

## Ideas & Plans

Ideally, the plan is to eventually modularise the project so we can cover a lot more area.

- X11 Overlay ✓
- Wayland/wlroots Overlay ✓
- OpenGL & Vulkan injectors 
- Gamescope specific mode (✓) with autostart (Systemd job?)
- CLI polling of Discord state ✓

## Plans to move over?

Currently there are no plans to move users of my previous projects to this one. Unless this really hits the ground running I do not expect it to reach feature parity much less improve.

## Installing

### From binaries

TBC

### From Package managers

TBC

### From Github source

Ensure you have `rust` and `cargo` installed.

```
git clone git@github.com:trigg/discern.git
cd discern
cargo run
```

#### Arch linux

```
pacman -S clang rustup
rustup default stable
```

## Did you really need to make another Discord overlay?

Technically it's not, yet.
