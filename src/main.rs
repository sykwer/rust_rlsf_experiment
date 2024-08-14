use core::slice;
use std::alloc::Layout;
use std::process;
use std::thread;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::{ptr, sync::atomic::AtomicBool};
use std::mem::MaybeUninit;
use libc::{mmap, munmap, PROT_READ, PROT_WRITE, MAP_PRIVATE, MAP_ANONYMOUS};
use std::sync::Arc;
use rlsf::Tlsf;

const HEAP_VIRT_SIZE: usize = 1024 * 1024 * 1024 * 30;
const PAGE_SIZE: usize = 4096;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let size = HEAP_VIRT_SIZE * std::mem::size_of::<u8>();
    let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);

    let ptr = unsafe {
        mmap(ptr::null_mut(), aligned_size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)
    };

    if ptr == libc::MAP_FAILED {
        return Err("mmap failed".into());
    }

    let array: &mut [MaybeUninit<u8>] = unsafe {
        std::slice::from_raw_parts_mut(ptr as *mut MaybeUninit<u8>, HEAP_VIRT_SIZE)
    };

    /*
    array[0].write(1);

    let value = unsafe {
        array[0].assume_init()
    };

    println!("First value: {}", value);
    */

    let mut tlsf: Tlsf<'_, u32, u32, 30, 16> = Tlsf::new();
    tlsf.insert_free_block(array);

    let sz = 100 * 1024 * 1024;

    unsafe {
        let layout = Layout::from_size_align(sz, 8).unwrap();
        let ptr = tlsf.allocate(layout).unwrap();
        let memory = slice::from_raw_parts_mut(ptr.cast::<u8>().as_ptr(), sz);

        for (index, byte) in memory.iter_mut().enumerate() {
            *byte = (index % 256) as u8;
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        let pid = process::id();
        let tid = thread::current().id();
        println!("PID: {}, TID: {:?}", pid, tid);
        thread::sleep(Duration::from_secs(1));
    }

    unsafe {
        munmap(ptr, aligned_size);
    }

    Ok(())
}
