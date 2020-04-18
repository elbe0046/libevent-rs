
use std::time::Duration;
use libevent::Libevent;
use libevent_sys;

pub mod ffi;

use std::os::raw::{c_int, c_short, c_void};

type EvutilSocket = c_int;
type EventCallbackFn = extern "C" fn(EvutilSocket, c_short, *mut c_void);

extern "C" fn hello_callback(fd: EvutilSocket, event: c_short, ctx: *mut c_void) {
    println!("Rust callback says hello");
}

fn main() {
    println!("Hello, world!");

    let mut libevent = Libevent::new()
        .unwrap_or_else(|e| panic!("{:?}", e));

    let _ = unsafe { libevent.with_base(|base| {
        ffi::helloc_init(base)
    })};

    let ev = unsafe { libevent.base_mut().event_new(
        None,
        libevent::EventFlags::PERSIST,
        hello_callback,
        unsafe {std::mem::transmute(std::ptr::null::<c_void>()) },
    ) };

    let _ = unsafe {
        libevent.base().event_add(&ev, Duration::from_secs(2))
    };

    let mut a: usize = 0;

    let _ev = libevent.add_interval(
        Duration::from_secs(6),
        move |_flags| {
            a += 1;
            println!("interval count: {}, flags: {:?}", a, _flags);
        }
    );

    for _count in 1..=3 {
        let now = std::time::Instant::now();
        libevent.run_timeout(Duration::from_secs(5));

        let elapsed = now.elapsed();

        println!("Ran libevent loop for {:?}", elapsed);
    }

    libevent.run();
}
