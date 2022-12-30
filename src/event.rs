use atomic_option::AtomicOption;
use std::{future::Future, pin::Pin, ptr::addr_of_mut, sync::atomic::AtomicBool, task::*};

use r_efi::{
    efi::{BootServices, Status},
    system::{EVT_NOTIFY_SIGNAL, TPL_CALLBACK},
};

// Userdata is shared between the mainloop and the callbacks, so it needs to be `Sync`.
struct Userdata {
    waker: AtomicOption<Waker>,
    signalled: AtomicBool,
}

impl Userdata {
    fn new() -> Box<Userdata> {
        Box::new(Userdata {
            waker: AtomicOption::empty(),
            signalled: AtomicBool::new(false),
        })
    }

    fn signal(&self) {
        self.signalled
            .store(true, core::sync::atomic::Ordering::Release);
        match &self.waker.take(std::sync::atomic::Ordering::Release) {
            Some(waker) => waker.wake_by_ref(),
            None => (),
        }
    }

    fn as_mut_ptr(&self) -> *mut std::ffi::c_void {
        self as *const Self as *mut std::ffi::c_void
    }

    fn is_signalled(&self) -> bool {
        self.signalled.load(core::sync::atomic::Ordering::Acquire)
    }

    fn set_waker(&self, waker: Waker) {
        // TODO: support replacing the waker
        self.waker
            .try_store(Box::new(waker), std::sync::atomic::Ordering::Release);
    }
}

pub struct Event {
    boot_services: *mut std::ffi::c_void,
    event: r_efi::efi::Event,
    data: Box<Userdata>,
}

impl Event {
    extern "win64" fn callback(_: r_efi::efi::Event, event: *mut core::ffi::c_void) {
        let cb = unsafe { &*(event as *const Userdata) };
        cb.signal()
    }

    pub fn new(bs: *mut BootServices) -> Result<Event, Status> {
        let mut event = std::ptr::null_mut();
        let data = Userdata::new();
        let r = unsafe {
            ((*bs).create_event)(
                EVT_NOTIFY_SIGNAL,
                TPL_CALLBACK,
                Some(Event::callback),
                data.as_mut_ptr(),
                addr_of_mut!(event),
            )
        };

        if r.is_error() {
            Err(r)
        } else {
            Ok(Event {
                boot_services: bs as *mut std::ffi::c_void,
                event,
                data,
            })
        }
    }

    pub fn get_raw_event(&self) -> r_efi::efi::Event {
        self.event
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        let bs = unsafe { &*(self.boot_services as *mut BootServices) };
        let r = (bs.close_event)(self.event);
        if r.is_error() {
            panic!("could not close event")
        }
        self.event = std::ptr::null_mut();
    }
}

impl Future for Event {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.data.set_waker(cx.waker().clone());
        match self.data.is_signalled() {
            false => Poll::Pending,
            true => Poll::Ready(()),
        }
    }
}
