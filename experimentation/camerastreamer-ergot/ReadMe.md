## Building

### Windows + MSYS2 ucrt64

This requires using the GNU rust toolchain, since pacman will install `.dll.a` files which are for the GNU linker.

```
$ pacman -S mingw-w64-ucrt-x86_64-clang
$ pacman -S mingw-w64-ucrt-x86_64-cmake
$ pacman -S mingw-w64-ucrt-x86_64-opencv
```

### Windows

#### Install CMake

Download .zip file version from here:
https://cmake.org/download/

extract it to `D:\Programs\cmake\cmake-4.2.0-rc1-windows-x86_64` so you have a `bin` folder.

create environment variable `CMAKE_HOME` with value `D:\Programs\cmake\cmake-4.2.0-rc1-windows-x86_64`

add the `%CMAKE_HOME%\bin` to the `PATH` environment variable.

#### Install vcpkg and ninja

https://learn.microsoft.com/en-gb/vcpkg/get_started/get-started?pivots=shell-powershell

From a command prompt (non-admin):

```
cd D:\Programs
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg
bootstrap-vcpkg.bat
vcpkg --version
vcpkg install vcpkg-tool-ninja
```

Log:
```
D:\Programs>cd vcpkg

D:\Programs\vcpkg>bootstrap-vcpkg.bat
Downloading https://github.com/microsoft/vcpkg-tool/releases/download/2025-10-16/vcpkg.exe -> D:\Programs\vcpkg\vcpkg.exe... done.
Validating signature... done.

vcpkg package management program version 2025-10-16-71538f2694db93da4668782d094768ba74c45991
```

create environment variable `VCPKG_HOME` with value `D:\Programs\vcpkg`

add the `%VCPKG_HOME%` to the `PATH` environment variable.

#### Install OpenCV

##### Via vcpkg

```
set VCPKG_BUILD_TYPE=release
vcpkg install opencv[contrib,world]:x64-windows
```
Note: `world` option generates one big .dll file.

corresponding .dll files will be in: `D:\Programs\vcpkg\installed\x64-windows\bin`

to be able to use the debugger, a debug version is required.

```
cd triplets
copy x64-windows-release.cmake x64-windows-debug.cmake
```
edit it, change the `VCPKG_BUILD_TYPE release` to `VCPKG_BUILD_TYPE debug`

i.e.:
```
set(VCPKG_TARGET_ARCHITECTURE x64)
set(VCPKG_CRT_LINKAGE dynamic)
set(VCPKG_LIBRARY_LINKAGE dynamic)
set(VCPKG_BUILD_TYPE debug)
```
then
```
vcpkg install opencv[contrib,world]:x64-windows-debug
```

or for static build (for .lib files)
```
vcpkg install opencv[contrib,world]:x64-windows-static
```

##### Issues

###### Error `(exit code: 0xc0000135, STATUS_DLL_NOT_FOUND)`

When using release mode non-static linking on builds work fine, but debug builds fail with this error.

for a working debug build this environement variable needs to be set prior to building:

`OPENCV_DISABLE_PROBES=vcpkg_cmake`

Reference: https://github.com/twistedfall/opencv-rust/issues/307

##### Manually

https://opencv.org/get-started/
https://github.com/opencv/opencv/releases/tag/4.12.0

download the C++ version of OpenCV, zip file, extract it.

Set OpenCV environment variables:

```
OPENCV_LINK_LIBS=static=opencv_world411,static=OpenCL
OPENCV_LINK_PATHS=D:\Users\Hydra\Documents\dev\projects\makerpnp\opencv\opencv-4.12\build\x64\vc16\lib
OPENCV_INCLUDE_PATHS=D:\Users\Hydra\Documents\dev\projects\makerpnp\opencv\opencv-4.12\build\include
```

### MacOS

```
cargo install cargo-bundle
brew install opencv
export DYLD_FALLBACK_LIBRARY_PATH="$(xcode-select --print-path)/usr/lib/"
cargo bundle --release
codesign --force --deep --sign - ./target/release/bundle/osx/CameraStreamer-Ergot.app
open ./target/release/bundle/osx
```
Then double-click the `CameraStreamer-Ergot.app` app, at which point is should request permission to access the camera.
