# Change log

NOTE: The major version number of this library matches that of the egui major version that this library currently supports. The minor version number may be different though. 

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