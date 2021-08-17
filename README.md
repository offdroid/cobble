# cobble

A basic Minecraft-esque voxel building game written in Rust based on the Bevy engine

![Screenshot 1](./assets/screenshots/01.jpg?raw=true "Screenshot 1")

<details>
<summary>More screenshots</summary>

![Screenshot 2](./assets/screenshots/02.jpg?raw=true "Screenshot 2")
![Screenshot 3](./assets/screenshots/03.jpg?raw=true "Screenshot 3")
</details>

## Features

- Block placement and destruction
- Basic physics based movement and collisions
- Procedural world generation
- Nine types of blocks

## Limitations

- No world persistence
- No async and/or parallel chunk loading and generation
- Movement can occasionally be a bit weird

## Running cobble

Run just like any other project with cargo
```bash
cargo run --release
```
For development, it might be beneficial to use the nightly toolchain with a special config, as detailed in the official bevy documentation, to drastically reduce the compile time.

Moreover, runtime options can be set in the `cobble.yaml`-file, like this
```yaml
video:
  vsync: false
game: 
  creative: false
# ...
```
<details>
<summary>Full schema (except for bindings, see defaults)</summary>
<p>

```yaml
debug:
  log_diagnostics: bool # default = false
  print_default_config: bool # default = false
  show_colliders: bool # default = false
  show_fps: bool # default = true
  show_selection: bool # default = true
  show_selection_normal: bool # default = false
game: 
  breakable_bedrock: false # default = false
  creative: true # default = true
input:
  bindings:
    # omitted, see default values for inspiration
  initial_cursor_grab: bool # default = true
  sensitivity: f32 # default = 1.0
video:
  msaa_samples: u32 # Any power of two, default = 4
  show_interface: bool # default = true
  vsync: bool # default = true
  window_mode: Windowed | Borderless | Fullscreen # default = Windowed
```
</p>
</details>

<details>
<summary>Default configuration</summary>
<p>

```yaml
debug: 
  log_diagnostics: false
  print_default_config: false
  show_colliders: false
  show_fps: true
  show_selection: true
  show_selection_normal: false
game: 
  breakable_bedrock: false
  creative: true
input: 
  bindings: 
    DeadZone: {}
    EventPhase: 
      BREAK: OnBegin
      FULLSCREEN_TOGGLE: OnBegin
      PLACE: OnBegin
    GamepadAxis: {}
    GamepadButtons: {}
    KeyboardKeys: 
      A: MOVE_LEFT
      D: MOVE_RIGHT
      F3: FULLSCREEN_TOGGLE
      Key1: SLOT_1
      Key2: SLOT_2
      Key3: SLOT_3
      Key4: SLOT_4
      Key5: SLOT_5
      Key6: SLOT_6
      Key7: SLOT_7
      Key8: SLOT_8
      Key9: SLOT_9
      LControl: MOVE_MOD_FAST
      LShift: MOVE_MOD_SLOW_DESC
      S: MOVE_BACKWARD
      Space: MOVE_JUMP
      Tab: FLY_TOGGLE
      W: MOVE_FORWARD
    MouseButtons: 
      Left: BREAK
      Middle: PICK_BLOCK
      Right: PLACE
    MouseMove: {}
  initial_cursor_grab: true
  sensitivity: 1.0
video: 
  msaa_samples: 4
  show_interface: true
  vsync: true
  window_mode: Windowed
```
</p>
</details>

<details>
<summary>Default key-bindings</summary>
  
| Action                               | Binding                                              | Note                              |
|--------------------------------------|------------------------------------------------------|-----------------------------------|
| Movement                             | <kbd>W</kbd>/<kbd>A</kbd>/<kbd> S</kbd>/<kbd>D</kbd> |                                   |
| Jump/Ascend                          | <kbd>Space</kbd>                                     | Alternative action only in flight |
| Sneak/Descend                        | <kbd>L-Shift</kbd>                                   | Alternative action only in flight |
| Sprint                               | <kbd>L-Control</kbd>                                 |                                   |
| Toggle fly                           | <kbd>Tab</kbd>                                       | Creative-mode only                |
| Pause                                | <kbd>ESC</kbd>                                       |                                   |
| Place block                          | <kbd>Right Mouse Button</kbd>                        |                                   |
| Break block                          | <kbd>Left Mouse Button</kbd>                         |                                   |
| Pick block to inventory              | <kbd>Middle Mouse Button</kbd>                       | Creative-mode only                |
| Switch active toolbar/inventory slot | <kbd>1</kbd> - <kbd>9</kbd>                          |                                   |
</details>

Cobble should run on most platforms, including WASM, but might require optimizations and adjustments to be usable on non-x86/x64 platforms or without keyboard- and mouse-input.

## License

Licensed under MIT, see [LICENSE](./LICENSE).
The licenses of the utilized code and assets are located under [LICENSES](./LICENSES).

## References 
1. [Minecraft](https://www.minecraft.net/)
2. [Bevy](https://bevyengine.org/) game engine
3. [bevy_flycam](https://github.com/sburris0/bevy_flycam)
4. [bevy_prototype_character_controller](https://github.com/superdump/bevy_prototype_character_controller/)
5. [bevy_mod_picking](https://github.com/aevyrie/bevy_mod_picking/)
6. [Majercik, A., Crassin, C., Shirley, P. and McGuire, M., 2018. _A ray-box intersection algorithm and efficient dynamic voxel rendering_. Journal of Computer Graphics Techniques Vol, 7(3).](http://jcgt.org/published/0007/03/04/)
7. [bevy_prototype_inline_assets](https://github.com/emosenkis/bevy_prototype_inline_assets)
8. [Rapier](https://rapier.rs/) physics engine 
9. [Noise](https://github.com/razaekel/noise-rs)
10. [Filament](https://github.com/google/filament)
11. [Building-blocks](https://github.com/bonsairobo/building-blocks)
