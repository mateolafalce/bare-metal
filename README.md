# Bare metal rust device

VGA Menu operations:

+ CPU data
+ Draw a 320x200 image in video mode
+ Reboot
+ Shutdown

## Building

You can build the project by running:

```
cargo build
```

To create a bootable disk image from the compiled kernel, you need to install the [`bootimage`] tool.

```
cargo install bootimage
```

After installing, you can create the bootable disk image by running:

```
cargo bootimage
```

This creates a bootable disk image in the `target/x86_64-blog_os/debug` directory.

Please file an issue if you have any problems.

## Running

You can run the disk image in [QEMU](https://www.qemu.org/) through.

```
cargo run
```

QEMU and the [`bootimage`] tool need to be installed for this.

You can also write the image to an USB stick for booting it on a real machine. On Linux, the command for this is:

```
dd if=target/x86_64/debug/bootimage-bare-metal.bin of=/dev/sdX && sync
```

Where `sdX` is the device name of your USB stick. **Be careful** to choose the correct device name, because everything on that device is overwritten.

## Creating an 320x200 image to print

You could do it with this (in a std enviroment)

```rust
use image::{GenericImageView, ImageReader};
use rgb2vga::rgb2vga;

fn main() {
    let img = ImageReader::open("input.png").unwrap().decode().unwrap();
    let img = img.resize_exact(320, 200, image::imageops::FilterType::Nearest);

    let mut raw_data = Vec::new();
    for (_, _, pixel) in img.pixels() {
        let vga_color = rgb2vga((pixel[0], pixel[1], pixel[2]));
        raw_data.push(vga_color);
    }

    std::fs::write("output.bin", raw_data).unwrap();
}
```