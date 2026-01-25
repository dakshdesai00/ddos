use core::ptr::{read_volatile, write_volatile};

const MBOX_BASE: usize = 0xFE00B880;
const MBOX_READ: *mut u32 = (MBOX_BASE + 0x00) as *mut u32;
const MBOX_STATUS: *mut u32 = (MBOX_BASE + 0x18) as *mut u32;
const MBOX_WRITE: *mut u32 = (MBOX_BASE + 0x20) as *mut u32;

const MBOX_FULL: u32 = 0x80000000;
const MBOX_EMPTY: u32 = 0x40000000;

#[repr(C, align(16))]
pub struct MboxMessage {
    pub data: [u32; 36],
}

pub struct Mailbox;

impl Mailbox {
    pub fn new() -> Mailbox {
        Mailbox
    }

    pub fn call(&self, channel: u32, msg: &mut MboxMessage) -> Result<(), ()> {
        let ptr = msg.data.as_ptr() as u32;
        if ptr & 0xF != 0 {
            return Err(());
        }

        let val = (ptr & !0xF) | (channel & 0xF);

        unsafe {
            while (read_volatile(MBOX_STATUS) & MBOX_FULL) != 0 {}
            write_volatile(MBOX_WRITE, val);

            loop {
                while (read_volatile(MBOX_STATUS) & MBOX_EMPTY) != 0 {}
                let response = read_volatile(MBOX_READ);
                if response == val {
                    return if msg.data[1] == 0x80000000 {
                        Ok(())
                    } else {
                        Err(())
                    };
                }
            }
        }
    }
}
