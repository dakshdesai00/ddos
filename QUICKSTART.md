# Quick Start Guide - DDOS Kernel Build & Run

## ⚡ TL;DR (For the Impatient)

### Build and Run in QEMU (Development)

```bash
./scripts/build-qemu.sh
```

**Done!** Kernel runs in QEMU. Exit with: `Ctrl+A` then `X`

### Build for Real RPi4

```bash
./scripts/build-rpi4.sh
```

**Follow the printed instructions** to flash to SD card.

### Build for Real RPi5

```bash
./scripts/build-rpi5.sh
```

**Generates SD boot files (`kernel_2712.img` + `config.txt`).**

---

## 📋 Requirements

### For QEMU Development

- Rust toolchain (nightly)
- ARM64 bare-metal target: `rustup target add aarch64-unknown-none-softfloat`
- QEMU: `brew install qemu`

### For Real Hardware

- Same as above
- Raspberry Pi 4
- Raspberry Pi 5
- microSD card
- Card reader

---

## 🚀 Step-by-Step: QEMU Development

### 1. Make script executable (first time only)

```bash
chmod +x ./scripts/build-qemu.sh
```

### 2. Run the script

```bash
./scripts/build-qemu.sh
```

### 3. Expected output

```
DDOS Kernel - QEMU Build & Run
========================================
[1/3] Cleaning previous build...
✓ Clean complete

[2/3] Building kernel for QEMU (RPi3 model)...
✓ Build complete

[3/3] Launching QEMU...
========================================
To exit QEMU: Press Ctrl+A then X
========================================

Welcome to DDOS Kernel v0.1
Testing Heap Allocation...
- Box allocated at 0x280018, value: 42
- Vec allocated: [0, 1, 2, 3, 4] (Success!)

>
```

### 4. Exit QEMU

Press `Ctrl+A` then `X`

---

## 🏴 Step-by-Step: Real RPi4 Hardware

### 1. Build the kernel

```bash
chmod +x ./scripts/build-rpi4.sh  # First time only
./scripts/build-rpi4.sh
```

### 2. Script output will show:

```
Build Successful!

Kernel binary at:
  target/aarch64-unknown-none-softfloat/debug/ddos

Next Steps: Flash to SD Card

1. Find your SD card device:
   diskutil list

2. Unmount SD card (replace 'X' with disk number):
   diskutil unmountDisk /dev/diskX

3. Flash kernel (replace 'X' with disk number):
   sudo dd if=target/aarch64-unknown-none-softfloat/debug/ddos ...
```

### 3. Find your SD card

```bash
diskutil list
```

Look for a device without "APPLE" in description, usually `/dev/disk2`, `/dev/disk3`, etc.

### 4. Unmount the SD card

```bash
diskutil unmountDisk /dev/diskX
```

Replace `X` with your disk number (e.g., `/dev/disk2`)

### 5. Flash the kernel

```bash
sudo dd if=target/aarch64-unknown-none-softfloat/debug/ddos of=/dev/rdiskX bs=4m
```

**Important:** Use `rdisk` (faster), not `disk`! Replace `X` with your number.

⚠️ **This will erase the disk!** Triple-check the disk number!

### 6. Eject safely

```bash
diskutil ejectDisk /dev/diskX
```

### 7. Power up the RPi4

1. Insert SD card into RPi4
2. Connect USB power
3. Connect UART serial cable to see boot messages
4. Watch the kernel boot!

---

## 🔧 Manual Build (If Scripts Don't Work)

### QEMU

```bash
cd /Users/dakshdesai/Codes/rust-os/ddos
cargo clean
cargo build --features qemu --target aarch64-unknown-none-softfloat
qemu-system-aarch64 -M raspi3b -serial stdio \
  -kernel target/aarch64-unknown-none-softfloat/debug/ddos
```

### RPi4

```bash
cd /Users/dakshdesai/Codes/rust-os/ddos
cargo clean
cargo build --no-default-features --features rpi4 \
            --target aarch64-unknown-none-softfloat
# Output: target/aarch64-unknown-none-softfloat/debug/ddos
# Then use dd command to flash
```

### RPi5

```bash
cd /Users/dakshdesai/Codes/rust-os/ddos
cargo clean
cargo build --no-default-features --features rpi5 \
            --target aarch64-unknown-none-softfloat
# Convert ELF to firmware image:
llvm-objcopy -O binary target/aarch64-unknown-none-softfloat/debug/ddos kernel_2712.img
# Copy kernel_2712.img + config.txt to mounted SD boot partition
```

---

## 🛠️ Troubleshooting

| Problem                                  | Solution                                                                 |
| ---------------------------------------- | ------------------------------------------------------------------------ |
| `qemu-system-aarch64: command not found` | `brew install qemu`                                                      |
| `target not found`                       | `rustup target add aarch64-unknown-none-softfloat`                       |
| `permission denied`                      | `chmod +x ./scripts/*.sh`                                                |
| QEMU won't exit                          | Press `Ctrl+A` then `X` (not `Q`!)                                       |
| RPi4 won't boot                          | Check: correct `rdisk` used, SD card fully powered, UART cable connected |
| `dd: command not found`                  | Only on Windows; use WSL or native Linux                                 |

---

## 📊 What Each Build Does

### `./scripts/build-qemu.sh`

1. ✅ Cleans old build
2. ✅ Builds with `--features qemu` (0x3F000000 peripheral base)
3. ✅ Launches QEMU emulator
4. ✅ Redirects serial to console (see kernel output directly)

### `./scripts/build-rpi4.sh`

1. ✅ Cleans old build
2. ✅ Builds with `--no-default-features --features rpi4` (0xFE000000 base)
3. ✅ Outputs binary ready for flashing
4. ✅ Prints flashing instructions

### `./scripts/build-rpi5.sh`

1. ✅ Cleans old build
2. ✅ Builds with `--no-default-features --features rpi5` (0x1F000000 base)
3. ✅ Converts ELF to `kernel_2712.img`
4. ✅ Generates `config.txt`
5. ✅ Optional auto-copy to mounted SD boot partition

---

## 🎯 Verifying the Build Works

You should see:

```
Welcome to DDOS Kernel v0.1
Testing Heap Allocation...
- Box allocated at 0x280018, value: 42
- Vec allocated: [0, 1, 2, 3, 4] (Success!)
```

This confirms:

- ✅ Kernel boots
- ✅ Heap allocator works
- ✅ Box and Vec work
- ✅ All drivers initialized

---

## 🔨 Build Output Location

- **Binary:** `target/aarch64-unknown-none-softfloat/debug/ddos`
- **Build logs:** Terminal output (or `cargo build ... 2>&1 | tee build.log`)
- **Target triple:** `aarch64-unknown-none-softfloat` (bare-metal ARM64)

---

## 📱 Hardware Info

### QEMU (Default Development)

- Emulates: Raspberry Pi 3B
- Peripheral base: 0x3F000000
- UART: 0x3F201000
- No physical hardware needed

### Real RPi4

- Raspberry Pi 4 Model B
- Peripheral base: 0xFE000000 (different from RPi3!)
- UART: 0xFE201000
- Needs microSD card, USB power, UART cable

### Real RPi5

- Raspberry Pi 5
- Peripheral base: 0x1F000000
- UART: 0x1F201000
- Needs microSD card, USB power, UART cable

---

## 💡 Pro Tips

### For Development Speed

- QEMU is fast! Use for testing
- Iterate quickly: `./scripts/build-qemu.sh`
- Only flash to real hardware when needed

### For Hardware Debugging

- Connect UART cable to see boot messages
- Use `screen /dev/tty.* 115200` to view serial output
- Build with debug symbols (default in dev profile)

### Building Different Targets

```bash
# Just build (don't run)
cargo build --features qemu --target aarch64-unknown-none-softfloat

# Build optimized
cargo build --features qemu --release --target aarch64-unknown-none-softfloat

# Check for errors only
cargo check --features qemu --target aarch64-unknown-none-softfloat
```

---

## 🔗 Related Documentation

- **Full Phase 1 Guide:** `Documentation/Phase1.md`
- **Script Details:** `scripts/README.md`
- **Build Summary:** `PHASE1_SUMMARY.md`
- **Hardware Selection:** `src/hardwareselect.rs`

---

**Last Updated:** February 21, 2026  
**Status:** ✅ Ready to Use
