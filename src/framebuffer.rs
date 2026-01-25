use crate::mailbox::{Mailbox, MboxMessage};
use core::ptr::write_volatile;

pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub base_addr: usize,
}

impl FrameBuffer {
    pub fn new() -> Result<FrameBuffer, u32> {
        let mut mbox = MboxMessage { data: [0; 36] };

        mbox.data[0] = 35 * 4; // Total size
        mbox.data[1] = 0; // Request

        let mut i = 2;
        // 1. Set Physical Size (1920 x 1080)
        mbox.data[i + 0] = 0x00048003;
        mbox.data[i + 1] = 8;
        mbox.data[i + 2] = 8;
        mbox.data[i + 3] = 1920;
        mbox.data[i + 4] = 1080;
        i += 5;

        // 2. Set Virtual Size (1920 x 1080)
        mbox.data[i + 0] = 0x00048004;
        mbox.data[i + 1] = 8;
        mbox.data[i + 2] = 8;
        mbox.data[i + 3] = 1920;
        mbox.data[i + 4] = 1080;
        i += 5;

        // 3. Set Virtual Offset (0, 0) - CRITICAL FOR QEMU RPI4
        mbox.data[i + 0] = 0x00048009;
        mbox.data[i + 1] = 8;
        mbox.data[i + 2] = 8;
        mbox.data[i + 3] = 0;
        mbox.data[i + 4] = 0;
        i += 5;

        // 4. Set Depth (32-bit)
        mbox.data[i + 0] = 0x00048005;
        mbox.data[i + 1] = 4;
        mbox.data[i + 2] = 4;
        mbox.data[i + 3] = 32;
        i += 4;

        // 5. Allocate Buffer
        mbox.data[i + 0] = 0x00040001;
        mbox.data[i + 1] = 8;
        mbox.data[i + 2] = 8;
        mbox.data[i + 3] = 4096;
        mbox.data[i + 4] = 0;
        i += 5;

        mbox.data[i] = 0; // End Tag

        let mb = Mailbox::new();
        // Check 1: Did the Mailbox transaction succeed?
        if mb.call(8, &mut mbox).is_err() {
            return Err(1); // Error Code 1: Mailbox Transport Failed
        }

        // Check 2: Did the GPU accept the tags? (Response code must be 0x80000000)
        if mbox.data[1] != 0x80000000 {
            return Err(mbox.data[1]); // Return the raw error code from GPU
        }

        // Get the pointer (Index 28 is where the pointer is stored in the Alloc Tag response)
        // Note: We added a tag (Offset), so the index shifted!
        // Header(2) + Phy(5) + Virt(5) + Offset(5) + Depth(4) + AllocHeader(3) = 24
        // So pointer is at data[24 + 3] = data[27]?
        // Let's count carefully:
        // Index 0, 1
        // Phy: 2,3,4,5,6
        // Virt: 7,8,9,10,11
        // Offset: 12,13,14,15,16
        // Depth: 17,18,19,20
        // Alloc: 21,22,23, (24=Alignment), (25=Size), (26=ADDRESS), (27=SIZE)

        // Wait, the Alloc Tag ID is at 21.
        // 21: ID, 22: Size, 23: Request/Resp Size
        // 24: Alignment (Request) / Address (Response) -- Wait, RPi firmware writes Address to the first value field.
        // So Address is at Index 24.

        let ptr_val = mbox.data[24];

        // FIX: Do NOT mask with 0x3FFFFFFF on RPi4 QEMU.
        // Trust the GPU. It usually returns 0x3E000000 or similar.
        let base_addr = ptr_val as usize;

        if base_addr == 0 {
            return Err(2); // Error Code 2: GPU returned Address 0
        }

        Ok(FrameBuffer {
            width: 1920,
            height: 1080,
            pitch: 1920 * 4,
            base_addr,
        })
    }

    pub fn draw_pixel(&self, x: u32, y: u32, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = (y * self.pitch) + (x * 4);
        unsafe {
            let addr = (self.base_addr + offset as usize) as *mut u32;
            write_volatile(addr, color);
        }
    }
}
