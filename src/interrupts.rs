use crate::{
    MENU, gdt, hlt_loop, println,
    vga_buffer::{BUFFER_HEIGHT, BUFFER_WIDTH, WRITER},
};
use alloc::string::{String, ToString};
use core::arch::asm;
use lazy_static::lazy_static;
use lscpu::Cpu;
use pic8259::ChainedPics;
use spin::{self, Mutex};
use vga::writers::Text80x25;
use vga::writers::TextWriter;
use vga::writers::{Graphics320x200x256, GraphicsWriter};
use x86_64::{
    instructions::port::Port,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

static IMAGE_DATA: &[u8] = include_bytes!("../output.bin");

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;
const MENU_RANGE: (u8, u8) = (20, 23);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
    static ref INDEX_MENU: Mutex<u8> = Mutex::new(MENU_RANGE.0);
    static ref WAIT_ENTER: Mutex<bool> = Mutex::new(false);
    static ref VIDEO_CHANGE: Mutex<bool> = Mutex::new(false);
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{
        DecodedKey, HandleControl, KeyCode, KeyEvent, KeyState, Keyboard, ScancodeSet1, layouts,
    };
    use spin::Mutex;
    use x86_64::instructions::port::Port;

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore
            ));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let other_case = KeyEvent::new(KeyCode::F12, KeyState::SingleShot);

    let scancode: u8 = unsafe { port.read() };
    let key_event_any = match keyboard.add_byte(scancode) {
        Ok(key_event) => key_event,
        Err(_) => Some(other_case.clone()),
    };

    let key_event = match key_event_any {
        Some(key) => key,
        None => other_case,
    };

    let key = match keyboard.process_keyevent(key_event) {
        Some(key) => key,
        None => pc_keyboard::DecodedKey::RawKey(KeyCode::F12),
    };

    match key {
        DecodedKey::Unicode(key) => {
            let mut wait = WAIT_ENTER.lock();
            let mut video_change = VIDEO_CHANGE.lock();
            clear_screen();
            if key == '\n' && !(*wait) {
                let index = INDEX_MENU.lock();
                match *index {
                    20 => {
                        cpu_info();
                        *wait = true;
                    }
                    21 => {
                        print_image();
                        *video_change = true;
                        *wait = true;
                    }
                    22 => reboot(),
                    23 => turn_off(),
                    _ => (),
                }
            } else if key == '\n' && *wait && !(*video_change) {
                println!("{MENU}");
                *wait = false;
            } else if key == '\n' && *wait && *video_change {
                let text_mode = Text80x25::new();
                text_mode.set_mode();
                disable_cursor();
                clear_screen();
                println!("{MENU}");

                let mut index_menu = INDEX_MENU.lock();
                *index_menu = MENU_RANGE.0;
                *video_change = false;
                *wait = false;
            }
        }
        DecodedKey::RawKey(key) => match key {
            KeyCode::ArrowUp => {
                let mut index_menu = INDEX_MENU.lock();
                if *index_menu > MENU_RANGE.0 {
                    *index_menu -= 1;
                }
                let mut writer = WRITER.lock();
                for i in 0..=(MENU_RANGE.1 - MENU_RANGE.0) {
                    let current_index_for_print: u8 = MENU_RANGE.0 + i;
                    if current_index_for_print == *index_menu {
                        writer.write_char_at(1, current_index_for_print.into(), b'*');
                    } else {
                        writer.write_char_at(1, current_index_for_print.into(), b' ');
                    }
                }
            }
            KeyCode::ArrowDown => {
                let mut index_menu = INDEX_MENU.lock();
                if *index_menu < MENU_RANGE.1 {
                    *index_menu += 1;
                }
                let mut writer = WRITER.lock();
                for i in 0..=(MENU_RANGE.1 - MENU_RANGE.0) {
                    let current_index_for_print: u8 = MENU_RANGE.0 + i;
                    if current_index_for_print == *index_menu {
                        writer.write_char_at(1, current_index_for_print.into(), b'*');
                    } else {
                        writer.write_char_at(1, current_index_for_print.into(), b' ');
                    }
                }
            }
            _ => (),
        },
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

pub fn clear_screen() {
    let blank = [b' '; BUFFER_WIDTH];
    let blank_string = String::from_utf8_lossy(&blank).to_string();
    for _ in 0..BUFFER_HEIGHT {
        println!("{}", blank_string);
    }
}

pub fn disable_cursor() {
    unsafe {
        let mut vga_index = Port::<u8>::new(0x3D4);
        let mut vga_data = Port::<u8>::new(0x3D5);
        vga_index.write(0x0A);
        vga_data.write(0x20);
    }
}

pub fn reboot() -> ! {
    unsafe {
        loop {
            outb(0x64, 0xFE);
        }
    }
}

unsafe fn outb(port: u16, val: u8) {
    unsafe {
        asm!("out dx, al", in("dx") port, in("al") val);
    }
}

unsafe fn outw(port: u16, val: u16) {
    unsafe {
        asm!("out dx, ax", in("dx") port, in("ax") val);
    }
}

fn turn_off() {
    unsafe {
        outw(0x604, 0x2000);
    }
}

fn cpu_info() {
    println!("{}", Cpu::new());
    println!("PRESS ENTER TO CONTINUE");
}

fn print_image() {
    let mode = Graphics320x200x256::new();
    mode.set_mode();
    let mut i = 0;
    for x in 0..200 {
        for y in 0..320 {
            let color = IMAGE_DATA[i];
            mode.set_pixel(y, x, color);
            i += 1;
        }
    }
}
