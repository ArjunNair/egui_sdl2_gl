# Change log

NOTE: The major version number of this library matches that of the egui major version that this library currently supports. The minor version number may be different though. 

# v0.26.2
* Updated to egui v0.26 and refactored some stuff. Thanks [David Cohen](https://github.com/osimarr)

# v0.23.0
* Updated to egui v0.23.0
* Fixed mix example throwing error due to Image api changes
* Fix for SDL2 not returning DPI correctly on VMs. Thanks [CarlosEduardoL](https://github.com/CarlosEduardoL)
* Better DPI default scaling that works on both Windows and Mac. Haven't been able to check on Linux, so if it's broken please let me know!

# v0.22.1
* Moved SDL 2 "bundled" feature to default-features to prevent compile issues on
  Mac. Thanks [aspect](https://github.com/aspect)
  
# v0.22.0
* Updated to egui v0.22.0. Thanks [Sean Ballais](https://github.com/seanballais)
  
# v0.16.0
* Updated to egui v0.16. Thanks [FireFlightBoy](https://github.com/FirelightFlagboy)

# v0.15.0
* Updated to egui v0.15.0
* Fix correct window not being checked for other events. See [issue](https://github.com/ArjunNair/egui_sdl2_gl/issues/11). Thanks [Yamakaky](https://github.com/Yamakaky)
* Added CI checks for clippy, rustfmt, etc. Thanks [Guillaume Gomez](https://github.com/GuillaumeGomez/)
* Re-export painter as pub. Thanks [d10sfan](https://github.com/d10sfan)
* Fix when keycode is None in keyboard event handling. Thanks [d10sfan](https://github.com/d10sfan)
* Accepted some clippy suggestions for simpler/better code. A couple of others I didn't understand. Suggestions welcome! :)

# v0.14.1
* The full egui demo lib has been added to examples + cleanup of examples + refactoring. Thanks [Adia Robbie](https://github.com/Ar37-rs).
* SDL2 bundled has been made optional again. Plus other SDL 2 features are now available as options. Thanks [Guillaume Gomez](https://github.com/GuillaumeGomez/).
* Fixed build on doc.rs + other fixes GL related fixes. Thanks [Guillaume Gomez](https://github.com/GuillaumeGomez/).
* Fixed correct window not being checked for window resize events. See [issue](https://github.com/ArjunNair/egui_sdl2_gl/issues/11). Thanks [Yamakaky](https://github.com/Yamakaky)

# v0.14.0
* Updated to egui v0.14.2
* Updated README to reflect SDL2 bundle feature introduced in v0.13.1

# v0.13.1
* Updated to egui v0.13.1
* Re-export dependencies Thanks [Guillaume Gomez](https://github.com/GuillaumeGomez/).
* Switched to SDL2 as a bundled feature. Updated to 0.34.5.

# v0.10.1
* Clipboard is now an optional feature that is enabled by default. Thanks [Katyo](https://github.com/katyo) 
* Fix for vertex array not being managed correctly. Thanks [FrankvdStam](https://github.com/FrankvdStam) 

# v0.10.0
* Fixed SRGB to linear color conversion.
* Fixed shader error on Mac
* Fixed triangle example bounds error when amplitude is too high.
* Updated to egui v0.10.0

# v0.1.9
* Made the background clear optional in Painter. This allows mixing custom Open GL draw calls with egui.
* Added an OpenGL Triangle example to demonstrate the above.
* Minor house keeping.

# v0.1.8
* Updated to egui 0.9.0.
* Better key input and text handling.
* Added cut, copy and paste support to the backend.
* Updated screenshot
* Fixed example sine wave speeding up unexpectedly after some time.

# v0.1.7
Updated to egui 0.8.0

# v0.1.6 
Changed OpenGL version to 3.2 as minimum to support Mac OS Catalina. GLSL to v1.50

# v0.1.5
Updated to egui 0.6.0

# v0.1.4
Updated to egui 0.5.0

# v0.1.3
Fixed dodgy modifier key check.

# v0.1.2
Bumped up egui dependency crate to 0.4 (latest as on Dec 13, 2020)
Fixed example to conform to egui 4.0 changes

# v0.1.1
Fix example.rs to use egui_sdl2_gl reference instead of egui_sdl2.
Added example screenshot to README.md.

# v0.1.0
First release.
