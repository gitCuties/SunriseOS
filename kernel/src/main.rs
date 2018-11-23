//! KFS
//!
//! A small kernel written in rust for shit and giggles. Also, hopefully the
//! last project I'll do before graduating from 42 >_>'.
//!
//! Currently doesn't do much, besides booting and printing Hello World on the
//! screen. But hey, that's a start.

#![feature(lang_items, start, asm, global_asm, compiler_builtins_lib, naked_functions, core_intrinsics, const_fn, abi_x86_interrupt, allocator_api, alloc, box_syntax, no_more_cas, const_vec_new, range_contains)]
#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]
#![allow(unused)]
#![recursion_limit = "1024"]

#[cfg(not(target_os = "none"))]
use std as core;

extern crate arrayvec;
extern crate bit_field;
#[macro_use]
extern crate lazy_static;
extern crate multiboot2;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate static_assertions;
#[macro_use]
extern crate alloc;
extern crate linked_list_allocator;
#[macro_use]
extern crate log;
extern crate smallvec;
extern crate hashmap_core;
extern crate xmas_elf;
extern crate rustc_demangle;
extern crate byteorder;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate bitfield;
extern crate kfs_libkern;

use core::fmt::Write;
use alloc::prelude::*;

mod paging;
mod event;
mod error;
mod logger;
mod log_impl;
pub use logger::*;
pub use devices::rs232::SerialLogger;
#[macro_use]
mod i386;
#[cfg(target_os = "none")]
mod gdt;
mod interrupts;
mod frame_allocator;

mod heap_allocator;
mod io;
mod devices;
mod sync;
mod process;
mod scheduler;
mod mem;
mod ipc;
mod elf_loader;
mod utils;
mod checks;

// Make rust happy about rust_oom being no_mangle...
pub use heap_allocator::rust_oom;

#[global_allocator]
static ALLOCATOR: heap_allocator::Allocator = heap_allocator::Allocator::new();

use i386::stack;
use paging::{PAGE_SIZE, MappingFlags};
use mem::{PhysicalAddress, VirtualAddress};
use paging::lands::{KernelLand, UserLand};
use process::{ProcessStruct, ThreadStruct, ThreadState};
use core::sync::atomic::Ordering;

unsafe fn force_double_fault() {
    loop {
        asm!("push 0" :::: "intel", "volatile");
    }
}

fn main() {
    info!("Loading all the init processes");
    for module in i386::multiboot::get_boot_information().module_tags().skip(1) {
        info!("Loading {}", module.name());
        let mapped_module = elf_loader::map_grub_module(module);
        let proc = ProcessStruct::new(String::from(module.name()), elf_loader::get_iopb(&mapped_module));
        let (ep, sp) = {
                let mut pmemlock = proc.pmemory.lock();

                let ep = elf_loader::load_builtin(&mut pmemlock, &mapped_module);

                let stack = pmemlock.find_available_space(5 * PAGE_SIZE)
                    .expect(&format!("Cannot create a stack for process {:?}", proc));
                pmemlock.guard(stack, PAGE_SIZE).unwrap();
                pmemlock.create_regular_mapping(stack + PAGE_SIZE, 4 * PAGE_SIZE, MappingFlags::u_rw()).unwrap();

                (VirtualAddress(ep), stack + 5 * PAGE_SIZE)
        };
        let thread = ThreadStruct::new(&proc, ep, sp)
            .expect("failed creating thread for service");
        ThreadStruct::start(thread)
            .expect("failed starting thread for service");
    }

    let lock = sync::SpinLockIRQ::new(());
    loop {
        // TODO: Exit process.
        scheduler::unschedule(&lock, lock.lock());
    }
}

#[cfg(target_os = "none")]
#[no_mangle]
pub unsafe extern fn start() -> ! {
    asm!("
        // Memset the bss. Hopefully memset doesn't actually use the bss...
        mov eax, BSS_END
        sub eax, BSS_START
        push eax
        push 0
        push BSS_START
        call memset
        add esp, 12

        // Save multiboot infos addr present in ebx
        push ebx
        call common_start" : : : : "intel", "volatile");
    core::intrinsics::unreachable()
}

/// CRT0 starts here.
#[cfg(target_os = "none")]
#[no_mangle]
pub extern "C" fn common_start(multiboot_info_addr: usize) -> ! {
    log_impl::early_init();

    // Register some loggers
    static mut SERIAL: SerialLogger = SerialLogger;
    Loggers::register_logger("Serial", unsafe { &mut SERIAL });


    let loggers = &mut Loggers;
    // Say hello to the world
    write!(Loggers, "\n# Welcome to ");
    loggers.print_attr("KFS",
                             LogAttributes::new_fg(LogColor::LightCyan));
    writeln!(Loggers, "!\n");

    // Parse the multiboot infos
    let boot_info = unsafe { multiboot2::load(multiboot_info_addr) };
    info!("Parsed multiboot informations");

    // Setup frame allocator
    frame_allocator::init(&boot_info);
    info!("Initialized frame allocator");

    // Set up (read: inhibit) the GDT.
    info!("Initializing gdt...");
    gdt::init_gdt();
    info!("Gdt initialized");

    i386::multiboot::init(boot_info);

    log_impl::init();

    unsafe { devices::pit::init_channel_0() };
    info!("Initialized PIT");

    info!("Enabling interrupts");
    unsafe { interrupts::init(); }

    //info!("Disable timer interrupt");
    //devices::pic::get().mask(0);

    info!("Becoming the first process");
    unsafe { scheduler::create_first_process() };

    info!("Calling main()");

    writeln!(SerialLogger, "= Calling main()");
    main();
    // Die !
    // We shouldn't reach this...
    loop {
        #[cfg(target_os = "none")]
        unsafe { asm!("HLT"); }
    }
}

#[cfg(target_os = "none")]
#[lang = "eh_personality"] #[no_mangle] pub extern fn eh_personality() {}

#[cfg(target_os = "none")]
#[panic_handler] #[no_mangle]
pub extern fn panic_fmt(p: &::core::panic::PanicInfo) -> ! {

    unsafe { Loggers.force_unlock(); }
    let _ = writeln!(Loggers, "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n\
                               ! Panic! at the disco\n\
                               ! {}\n\
                               !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
                     p);

    loop { unsafe { asm!("HLT"); } }
}
