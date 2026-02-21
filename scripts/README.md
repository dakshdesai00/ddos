# Build Scripts for DDOS Kernel

This directory contains automated build scripts for different target platforms.

## Available Scripts

### `build-qemu.sh` - QEMU Development Build

**Builds and runs the kernel in QEMU emulator (RPi3 model).**

```bash
./scripts/build-qemu.sh
```

**What it does:**
1. Cleans previous build artifacts
2. Compiles with `--features qemu` (peripheral base: 0x3F000000)
3. Launches QEMU with serial output to terminal

**To exit QEMU:** Press `Ctrl+A` then `X`

**Requirements:**
- `cargo` (Rust toolchain)
- `qemu-system-aarch64`

---

### `build-rpi4.sh` - Raspberry Pi 4 Hardware Build

**Builds kernel ready for flashing onto real RPi4 hardware.**

```bash
./scripts/build-rpi4.sh
```

**What it does:**
1. Cleans previous build artifacts
2. Compiles with `--features rpi4` (peripheral base: 0xFE000000)
3. Outputs binary ready for SD card flashing

**Output location:**
```
target/aarch64-unknown-none-softfloat/debug/ddos
```

**To flash to SD card:**

```bash
# 1. Find your SD card device
diskutil list

# 2. Unmount SD card (replace X with your disk number)
diskutil unmountDisk /dev/diskX

# 3. Flash kernel (use rdisk for speed, not disk!)
sudo dd if=target/aarch64-unknown-none-softfloat/debug/ddos of=/dev/rdiskX bs=4m

# 4. Eject safely
diskutil ejectDisk /dev/diskX

# 5. Insert SD card into RPi4 and power on!
```

**⚠️ WARNING:** Using the wrong disk number in `dd` will erase that disk! Be very careful with `diskutil list`.

---

## Manual Build (If Scripts Don't Work)

### For QEMU:
```bash
cd /Users/dakshdesai/Codes/rust-os/ddos
cargo clean
cargo build --features qemu --target aarch64-unknown-none-softfloat
qemu-system-aarch64 -M raspi3b -serial stdio -kernel target/aarch64-unknown-none-softfloat/debug/ddos
```

### For Real RPi4:
```bash
cd /Users/dakshdesai/Codes/rust-os/ddos
cargo clean
cargo build --features rpi4 --target aarch64-unknown-none-softfloat
# Then flash using dd command above
```

---

## Hardware Selection via Cargo Features

The build system uses Cargo features to select target hardware:

### `qemu` - QEMU Emulation (RPi3 Model)
- Peripheral base: `0x3F000000`
- Used by QEMU's `-M raspi3b` emulator
- Best for development and testing

### `rpi3` - Real Raspberry Pi 3
- Peripheral base: `0x3F000000`
- Same as QEMU (RPi3 is the target)
- For flashing to real RPi3 hardware

### `rpi4` - Real Raspberry Pi 4
- Peripheral base: `0xFE000000`
- Different from RPi3!
- For flashing to real RPi4 hardware

**Note:** Exactly ONE feature must be enabled at build time.

---

## Troubleshooting

### Script Doesn't Execute
```bash
# Make it executable
chmod +x ./scripts/build-qemu.sh
chmod +x ./scripts/build-rpi4.sh
```

### QEMU Not Found
```bash
# Install via Homebrew (macOS)
brew install qemu
```

### Build Fails
1. Ensure you have the Rust nightly toolchain:
   ```bash
   rustup toolchain install nightly
   ```

2. Add the ARM64 bare-metal target:
   ```bash
   rustup target add aarch64-unknown-none-softfloat
   ```

3. Try manual build to see full error messages:
   ```bash
   cargo build --features qemu --target aarch64-unknown-none-softfloat
   ```

---

## Testing the Build

After running either build script, you should see output like:

```
Welcome to DDOS Kernel v0.1
Testing Heap Allocation...
- Box allocated at 0x280018, value: 42
- Vec allocated: [0, 1, 2, 3, 4] (Success!)

> _
```

This confirms:
- ✅ Kernel boots successfully
- ✅ Heap allocator is working
- ✅ Box and Vec allocations succeed

---

## Build System Details

### Cargo.toml Features

```toml
[features]
qemu  = []  # Build for QEMU (RPi3 model, 0x3F000000 base)
rpi3  = []  # Build for real RPi3 (0x3F000000 base)
rpi4  = []  # Build for real RPi4 (0xFE000000 base)
```

### Compile-Time Checks

The build system includes compile-time verification that exactly one feature is enabled:

```rust
// In src/hardwareselect.rs
#[cfg(not(any(feature = "qemu", feature = "rpi3", feature = "rpi4")))]
compile_error!("Must enable exactly one hardware feature");

#[cfg(all(feature = "qemu", feature = "rpi3"))]
compile_error!("Cannot enable both 'qemu' and 'rpi3'");
```

This prevents configuration errors!

---

Last Updated: February 21, 2026
