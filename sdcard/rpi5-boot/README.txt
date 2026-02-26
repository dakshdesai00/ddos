RPi5 boot folder (copy-ready)

After running:
  ./scripts/build-rpi5.sh

This folder contains files to copy to the SD boot partition root:
- kernel_2712.img
- config.txt

Example (macOS):
  cp kernel_2712.img /Volumes/bootfs/kernel_2712.img
  cp config.txt /Volumes/bootfs/config.txt
