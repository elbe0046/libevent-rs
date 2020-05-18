#![allow(dead_code)]

use std::convert::From;
use std::io;
use std::os::raw::{c_int, c_short};
use std::time::Duration;
use libevent_sys;

mod event;
pub use event::*;

mod time;
pub use time::*;

/// Gets used as the boxed context for `EXternCallbackFn`
struct EventCallbackWrapper {
    inner: Box<dyn FnMut(EventHandleInner, EventFlags)>,
    ev: *mut libevent_sys::event,
}

extern "C" fn handle_wrapped_callback(_fd: EvutilSocket, event: c_short, ctx: EventCallbackCtx) {
    let cb_ref = unsafe {
        let cb: *mut EventCallbackWrapper = /*std::mem::transmute(*/ ctx as *mut EventCallbackWrapper/*)*/;
        let _cb_ref: &mut EventCallbackWrapper = &mut *cb;
        _cb_ref
    };

    let flags = EventFlags::from_bits_truncate(event as u32);
    let event_handle = EventHandleInner { inner: cb_ref.ev };
    (cb_ref.inner)(event_handle, flags)
}

pub struct Libevent {
    base: EventBase,
}

impl Libevent {
    pub fn new() -> Result<Self, io::Error> {
        EventBase::new()
            .map(|base| Libevent { base })
    }

    // TODO: This should be raw_base, and EventBase should prevent having to use raw altogether.
    /// # Safety
    /// Exposes the event_base handle, which can be used to make any sort of
    /// modifications to the event loop without going through proper checks.
    pub unsafe fn with_base<F: Fn(*mut libevent_sys::event_base) -> c_int>(
        &mut self,
        f: F
    ) -> c_int
        where
    {
        f(self.base.as_inner_mut())
    }

    /// # Safety
    /// Exposes the event_base handle, which can be used to make any sort of
    /// modifications to the event loop without going through proper checks.
    pub/*(crate)*/ unsafe fn base(&self) -> &EventBase {
        &self.base
    }

    /// # Safety
    /// Exposes the event_base handle, which can be used to make any sort of
    /// modifications to the event loop without going through proper checks.
    pub/*(crate)*/ unsafe fn base_mut(&mut self) -> &mut EventBase {
        &mut self.base
    }

    /// Turns the libevent base once.
    // TODO: any way to show if work was done?
    pub fn turn(&self) -> ExitReason {
        self.base.loop_(LoopFlags::NONBLOCK)
    }

    /// Turns the libevent base until exit or timeout duration reached.
    // TODO: any way to show if work was done?
    pub fn run_timeout(&self, timeout: Duration) -> ExitReason {
        if self.base.loopexit(timeout) != 0 {
            // TODO: This conflates errors, is it ok?
            return ExitReason::Error;
        };
        self.base.loop_(LoopFlags::empty())
    }

    /// Turns the libevent base until next active event.
    // TODO: any way to show if work was done?
    pub fn run_until_event(&self, timeout: Option<Duration>) -> ExitReason {
        if let Some(timeout) = timeout {
            if self.base.loopexit(timeout) != 0 {
                // TODO: This conflates errors, is it ok?
                return ExitReason::Error;
            }
        }
        self.base.loop_(LoopFlags::ONCE)
    }

    /// Turns the libevent base until exit.
    // TODO: any way to show if work was done?
    pub fn run(&self) -> ExitReason {
        self.base.loop_(LoopFlags::empty())
    }

    fn add_timer<F>(&mut self, tv: Duration, cb: F, flags: EventFlags) -> io::Result<EventHandle>
        where
            F: FnMut(EventHandleInner, EventFlags) + 'static
    {
        let mut ev = unsafe { self.base_mut().event_new(
            None,
            flags,
            handle_wrapped_callback,
            None,
        ) };

        let mut cb_wrapped = Box::new(EventCallbackWrapper {
            inner: Box::new(cb),
            ev: ev.inner.inner,
        });

        // libevent does similar shenanigans for `event_self_cbarg`, but we
        // need to use the context for both ev handle as well as the closure.
        // Thus, we first malloc'ed ev with mostly-bs values in `event_new`,
        // but set things for realsies in `event_assign` now that ev is set.
        let _ = unsafe { self.base_mut().event_assign(
            &mut ev,
            None,
            flags,
            handle_wrapped_callback,
            Some(std::mem::transmute(cb_wrapped)),
        ) };

        let _ = unsafe {
            self.base().event_add(&ev, tv)
        };

        Ok(ev)
    }

    pub fn add_interval<F>(&mut self, interval: Duration, cb: F) -> io::Result<EventHandle>
    where
        F: FnMut(EventHandleInner, EventFlags) + 'static
    {
        self.add_timer(interval, cb, EventFlags::PERSIST)
    }

    pub fn add_oneshot<F>(&mut self, timeout: Duration, cb: F) -> io::Result<EventHandle>
    where
        F: FnMut(EventHandleInner, EventFlags) + 'static
    {
        self.add_timer(timeout, cb, EventFlags::empty())
    }
}

impl From<EventBase> for Libevent {
    fn from(base: EventBase) -> Libevent {
        Libevent {
            base,
        }
    }
}
