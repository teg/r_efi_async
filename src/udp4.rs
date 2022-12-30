use r_efi::efi::{protocols::udp4::*, Status};

use crate::event::Event;

pub async fn transmit(
    boot_services: *mut r_efi::system::BootServices,
    protocol: *mut r_efi::protocols::udp4::Protocol,
    tx_data: *mut r_efi::protocols::udp4::TransmitData,
) -> Result<(), Status> {
    let event = Event::new(boot_services)?;

    let mut token = CompletionToken {
        event: event.get_raw_event(),
        status: Status::NOT_READY,
        packet: CompletionTokenPacket { tx_data },
    };

    let r = unsafe { ((*protocol).transmit)(protocol, &mut token) };
    if r.is_error() {
        return Err(r);
    }

    event.await;

    if token.status.is_error() {
        Err(token.status)
    } else {
        Ok(())
    }
}

struct ReceivedPacket {
    boot_services: *mut r_efi::system::BootServices,
    data: ReceiveData,
}

impl Drop for ReceivedPacket {
    fn drop(&mut self) {
        let r = unsafe { ((*self.boot_services).signal_event)(self.data.recycle_signal) };
        if r.is_error() {
            panic!("could not signal recycle")
        }
    }
}

pub async fn receive(
    boot_services: *mut r_efi::system::BootServices,
    protocol: *mut r_efi::protocols::udp4::Protocol,
) -> Result<ReceivedPacket, Status> {
    let event = Event::new(boot_services)?;

    let mut token = CompletionToken {
        event: event.get_raw_event(),
        status: Status::NOT_READY,
        packet: CompletionTokenPacket {
            rx_data: std::ptr::null_mut(),
        },
    };
    let r = unsafe { ((*protocol).receive)(protocol, &mut token) };
    if r.is_error() {
        return Err(r);
    }

    event.await;

    if token.status.is_error() {
        Err(token.status)
    } else {
        Ok(ReceivedPacket {
            boot_services,
            data: unsafe { *token.packet.rx_data },
        })
    }
}
