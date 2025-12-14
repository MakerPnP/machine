# Screen 13 - cli - 1

This experimentation attempts to use Screen13 to render a 3D scene to set of png files.

Also includes an unfinished attempt to render step files.

## Dependencies

### Windows

```
vcpkg install shaderc:x64-windows
```

### Linux

Tested on a Raspberry Pi 5.

```
$ vulkaninfo | grep version
Vulkan Instance Version: 1.4.309
        apiVersion        = 1.3.305 (4206897)
        driverVersion     = 25.0.7 (104857607)
        conformanceVersion:
        apiVersion        = 1.4.305 (4210993)
        driverVersion     = 0.0.1 (1)
        shaderBinaryVersion  = 1
        conformanceVersion:
```

```
apt install vulkan-tools mesa-vulkan-drivers libvulkan-dev glslc
```

## Building shaders

Required glslc compiler.

See the scripts in `dist`

## Running

```
cargo run --release
```

## Output

The program generates a png file called `asserts/cube_nnn.png` (the first one is shown below)

![output](assets/cube_001.png)

## Models

Step files come from here: https://www.3dcontentcentral.com/parts/part.aspx?id=263553&catalogid=171

* LQFP64 - by Michael Ludwig

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
