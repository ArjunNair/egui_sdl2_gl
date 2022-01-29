[![Crates.io](https://img.shields.io/crates/v/egui_sdl2_gl.svg)](https://crates.io/crates/egui_sdl2_gl)
[![Documentation](https://docs.rs/egui_sdl2_gl/badge.svg)](https://docs.rs/egui_sdl2_gl)
[![CI](https://github.com/ArjunNair/egui_sdl2_gl/actions/workflows/CI/badge.svg)](https://github.com/ArjunNair/egui_sdl2_gl/actions/workflows/ci.yaml)

# Egui backend for SDL2 + Open GL
![Example screenshot](/media/egui_sdl2_gl_example.png)

This is a backend implementation for [Egui](https://github.com/emilk/egui) that can be used with [SDL 2](https://github.com/Rust-SDL2/rust-sdl2) for events, audio, input et al and [OpenGL](https://github.com/brendanzab/gl-rs) for rendering.

I've included an example in the examples folder to illustrate how the three can be used together. To run the example, do the following:

```
cargo run --example basic
cargo run --example mix
cargo run --example demo_lib --features=use_epi
```

Starting with v13.1 SDL2 is 'bundled' as a cargo requirement and so SDL2 needn't be setup separately. If, however, you wish to be in control of the SDL2 setup, you can remove the bundled feature from the cargo.toml and set up the SDL2 framework separately, as described in the SDL2 repo above.

Note that using OpenGL involves wrapping **any**  Open GL call in an *unsafe* block. Have a look at the src/painter.rs file to see what I mean. This of course means that all bets are off when dealing with code inside the unsafe blocks, but that's the price to pay when dealing with raw OpenGL. 

Why would anyone want to use this then, you wonder? Well I would say the familiarity of using SDL2, the elegance of Egui and the power of OpenGL makes for a good combination in making games, emulators, graphics tools and such.

As far as the implementation goes, I've used Emil's original egui_glium and egui_web backends (see the egui github for source) as guides to implement this version, but have deviated in a couple of ways: 

1. It doesn't use the App architecture as used in the original code because I wanted to keep it as simple as possible. 
2. I've added a *update_user_texture_data* method to the painter class, which allows for easy dynamic updating of textures that need to be managed by Egui (to render in an Image control, say). See examples/example.rs to see how this can be useful.

I'm not an expert in Egui, Open GL or Rust for that matter. Please do submit an issue ticket (or better, send a PR!) if you spot something something that's out of whack in so far as the backend implementation goes. Issues regarding SDL2, Egui or OpenGL should be directed towards their respective repository owners!

Note: most of essential features are supported now.
