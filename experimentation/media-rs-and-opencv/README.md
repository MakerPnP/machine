<div align="center">

[![Discord](https://img.shields.io/discord/1255867192503832688?label=MakerPnP%20discord&color=%2332c955)](https://discord.gg/ffwj5rKZuf)
[![YouTube Channel Subscribers](https://img.shields.io/youtube/channel/subscribers/UClzmlBRrChCJCXkY2h9GhBQ?style=flat&color=%2332c955)](https://www.youtube.com/channel/UClzmlBRrChCJCXkY2h9GhBQ?sub_confirmation=1)
[![MakerPnP GitHub Organization's stars](https://img.shields.io/github/stars/makerpnp?style=flat&color=%2332c955)](https://github.com/MakerPnP)
[![Donate via Ko-Fi](https://img.shields.io/badge/Ko--Fi-Donate-green?style=flat&color=%2332c955&logo=ko-fi)](https://ko-fi.com/dominicclifton)
[![Subscribe on Patreon](https://img.shields.io/badge/Patreon-Subscribe-green?style=flat&color=%2332c955&logo=patreon)](https://www.patreon.com/MakerPnP)

![MakerPnP](assets/logos/makerpnp_icon_1_384x384.png)

</div>

# Video Capture and OpenCV

A cross-platform tool to display video streams concurrently with [OpenCV](https://docs.opencv.org/4.x/index.html) visualizations.

Video capture is done using the [`media-rs`](https://github.com/rust-media/media-rs) crate.

Recent screenshot:
[<img src="assets/screenshots/TODO.png" width="800" alt="VideoCapture + OpenCV">](assets/screenshots/TODO.png)

## Background

This was written as an experiment to see how to use the [`media-rs`](https://github.com/libark/video-capture) crate with a minimal set of `OpenCV` dependencies.

As of OpenCV 4.x there is no way to enumerate cameras with OpenCV itself, the API doesn't allow you to specify a camera id,
device path, serial number or other unique identifier when opening a video capture device.  This makes it unsuitable for
use when you want your program to always use the same cameras, regardless of which port it may be connected to.  Additionally, 
OpenCV doesn't have an API for discovering available camera resolutions, frame rates or video formats (YUYV, NV12, MJPEG, etc), 
this is usually required for a camera application.  It's highly recommend to use uncompressed formats like YUYV or NV12
 when doing machine vision work due to the compression artifacts that can be introduced by MJPEG, or similar compressed formats.

Thus, a solution was needed to capture video frames from a specific camera, using known-good resolution, frame rate and
video format and then feed the frames into OpenCV for visualizations.

There was also a desire to avoid the [`videoio`](https://github.com/opencv/opencv/blob/4.x/modules/videoio/doc/videoio_overview.markdown) feature in the OpenCV library since it uses much more C code and additional
unsafe C baggage.

Since this is an experimentation project, the code is not optimized for performance.  There may be more optimal ways to
get the camera images into OpenCV that what is shown here, currently the code converts the images into BGR `Mat`s for 
processing by OpenCV.

Other related experiments:
* `videocapture-and-opencv` (this supercedes it, since `video-capture` is older than `media-rs` and doesn't support capturing compressed video frames)
* `camera-enumeration-windows`

## Building

Requires OpenCV to be installed, follow the instructions here: https://github.com/twistedfall/opencv-rust and ensure the
pre-requisites are met, here: https://github.com/twistedfall/opencv-rust/blob/master/INSTALL.md

Currently only the following OpenCV modules are used: "imgcodecs", "imgproc", "objdetect", you may be able to build
opencv with only those modules if you prefer.  See this crate's `Cargo.toml` for the latest list of OpenCV modules used.

normally, build with:

```
cargo build --release
```

### Windows/MSYS2/UCRT64

```
cargo build --target x86_64-pc-windows-gnu --release
```
or
```
rustup run stable-x86_64-pc-windows-gnu cargo build --target x86_64-pc-windows-gnu --release
```

### Windows

Tested on 
a) Windows 11, with OpenCV built using vcpkg, using the `x86_64-windows-pc-msvc` toolchain.
b) Windows 11/MSYS2/UCRT64, with OpenCV installed using `pacman` using the `x86_64-pc-windows-gnu` target. 

## Running

OpenCV libraries must be available on the system path.

Linux/macOS:
```
$ ./target/release/videocapture-and-opencv
```

### Windows
```
> .\target\release\videocapture-and-opencv.exe
```

### Windows/MSYS2/UCRT64

```
$ ./target/x86_64-pc-windows-gnu/release/videocapture-and-opencv.exe
```

## OpenCV data files

For face detection, the OpenCV data files are required.

https://github.com/opencv/opencv/tree/4.x/data

Common paths below, the OpenCV path in the UI needs to be set to the same path as the data files, which should contain a `haarcascades` folder.

### Windows/MSYS2/UCRT64

```
C:\msys64\ucrt64\share\opencv4
```

### Windows/VCPKG

This depends on the exact version of OpenCV that was installed by vcpkg when you built OpenCV.

e.g.:
```
D:\Programs\vcpkg\buildtrees\opencv4\src\4.11.0-0357908e41.clean\data
```


## Donations

If you find this project useful, please consider making a donation via Ko-Fi or Patreon. 

* Ko-fi: https://ko-fi.com/dominicclifton
* Patreon: https://www.patreon.com/MakerPnP

## Links

Please subscribe to be notified of live-stream events so you can follow further developments.

* Patreon: https://www.patreon.com/MakerPnP
* Source: https://github.com/MakerPnP
* Discord: https://discord.gg/ffwj5rKZuf
* YouTube: https://www.youtube.com/@MakerPnP
* X/Twitter: https://x.com/MakerPicknPlace

## Authors

* Dominic Clifton - Project founder and primary maintainer.

## License

Dual-licensed under Apache or MIT, at your option.

## Contributing

If you'd like to contribute, please raise an issue or a PR on the github issue tracker, work-in-progress PRs are fine
to let us know you're working on something, and/or visit the discord server.  See the ![Links](#links) section above.
