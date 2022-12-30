#![no_main]

#[export_name = "efi_main"]
pub extern "C" fn main(_h: *mut core::ffi::c_void, system_table: *mut core::ffi::c_void) -> usize {
    let mut system_table = system_table as *mut r_efi::system::SystemTable;
    executor::block_on(async_main(system_table.boot_services));
    0
}

use r_efi::{
    protocols::udp4::{FragmentData, Protocol, TransmitData},
    system::BootServices,
};

mod event;
mod executor;
mod udp4;

async fn async_main(boot_services: *mut BootServices) {
    let mut protocol: *mut Protocol = std::ptr::null_mut();
    let r = unsafe {
        ((*boot_services).locate_protocol)(
            &mut r_efi::protocols::udp4::PROTOCOL_GUID,
            std::ptr::null_mut(),
            std::ptr::addr_of_mut!(protocol) as *mut *mut std::ffi::c_void,
        )
    };
    let mut tx_data = TransmitData {
        udp_session_data: std::ptr::null_mut(),
        gateway_address: std::ptr::null_mut(),
        data_length: 0,
        fragment_count: 0,
        fragment_table: [FragmentData {
            fragment_length: 0,
            fragment_buffer: std::ptr::null_mut(),
        }; 0],
    };
    udp4::transmit(boot_services, protocol, &mut tx_data).await;
    let packet = udp4::receive(boot_services, protocol).await;
}
