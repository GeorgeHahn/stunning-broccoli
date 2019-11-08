extern crate log;
extern crate nom;
extern crate rusb;
extern crate simple_logger;

use log::{info, trace};
use std::fmt::Debug;
use std::time::Duration;

mod magic;
mod packet;

use packet::*;

const HUB_VENDOR_ID: u16 = 0x1A86;
const HUB_PRODUCT_ID: u16 = 0xE024;

pub fn get_hubs(context: &rusb::Context) -> Vec<rusb::Device> {
    match context.devices() {
        Ok(devices) => {
            let mut hubs = vec![];
            for device in devices.iter() {
                if let Ok(device_desc) = device.device_descriptor() {
                    if device_desc.vendor_id() == HUB_VENDOR_ID
                        && device_desc.product_id() == HUB_PRODUCT_ID
                    {
                        hubs.push(device);
                    }
                }
            }
            return hubs;
        }
        Err(_) => return vec![],
    }
}

pub struct WyzeHub<'a> {
    handle: rusb::DeviceHandle<'a>,
    context: &'a rusb::Context,
}

impl<'a> WyzeHub<'a> {
    pub fn init(&mut self) {
        info!("Reset");
        self.handle.reset().unwrap();

        info!("Set active config");
        self.handle.set_active_configuration(0x00).unwrap();

        if let Ok(result) = self.handle.kernel_driver_active(0x00) {
            if result {
                info!("Kernel driver active! Detaching");
                self.handle.detach_kernel_driver(0x00).unwrap();
            }
        }

        info!("Claim interface");
        self.handle.claim_interface(0x00).unwrap();

        info!("USB HID setup complete");

        self.send(InquiryPacket);
        self.raw_read();

        self.send(GetMacPacket);
        self.raw_read();

        self.send(GetVerPacket);
        self.raw_read();

        self.send(GetSensorCountPacket);
        self.raw_read();

        self.send(GetSensorListPacket::create(5));
        self.raw_read();
        self.raw_read();
        self.raw_read();

        self.send(AuthPacket::create_done());

        info!("Hub setup complete");

        self.run();
    }

    fn send<P>(&self, packet: P)
    where
        P: Packet + Packable + Debug,
    {
        let mut write: Vec<u8> = Vec::new();
        let data = packet.to_bytes();
        trace!("Sending packet {:?}, {:?}", packet, data[0]);

        // Direction
        write.extend(&[0xAA, 0x55]);

        // Type
        match packet.get_packet_type() {
            PacketSyncType::Sync => write.push(0x43),
            PacketSyncType::Async => write.push(0x53),
        }

        // Length
        write.push(data.len() as u8 + 2);

        // payload
        write.extend(data);

        // checksum
        let ck: u16 = write
            .iter()
            .fold(0u16, |acc, x| acc.wrapping_add(*x as u16));
        let ck_bytes: &[u8] = &[(ck >> 8 & 0xFF) as u8, (ck & 0xFF) as u8];
        write.extend(ck_bytes);

        self.raw_write(write);
    }

    fn raw_write(&self, data: Vec<u8>) {
        self.handle
            .write_control(
                0x21,   // rusb_REQUEST_TYPE_CLASS | rusb_RECIPIENT_INTERFACE | rusb_ENDPOINT_OUT
                0x09,   // HID SET_REPORT
                0x02AA, // Report number 0xAA
                0x0000,
                &data,
                std::time::Duration::new(1, 0),
            )
            .unwrap();
    }

    fn raw_read(&mut self) {
        let timeout = Duration::from_secs(1);
        let mut rsv_bytes = vec![];
        let mut async_group = rusb::AsyncGroup::new(&self.context);

        async_group
            .submit(rusb::Transfer::interrupt(&self.handle, 0x82, timeout))
            .unwrap();

        loop {
            if let Some(mut transfer) = async_group.any().unwrap() {
                if transfer.status() == rusb::TransferStatus::Success {
                    rsv_bytes.extend_from_slice(transfer.actual());
                    break;
                }
                async_group.submit(transfer).unwrap();
            }
        }

        while !rsv_bytes.is_empty() {
            if let Ok((remaining, msg)) = magic::parse(&rsv_bytes) {
                let removed = rsv_bytes.len() - remaining.len();
                rsv_bytes = rsv_bytes[removed..].to_vec();
                info!("parsed {:?}", msg);
            } else {
                rsv_bytes.clear();
            }
        }
    }

    fn run(&mut self) {
        let timeout = Duration::from_secs(1);
        let mut rsv_bytes = vec![];
        let mut async_group = rusb::AsyncGroup::new(&self.context);
        let mut read_active = false;
        loop {
            if !read_active {
                async_group
                    .submit(rusb::Transfer::interrupt(&self.handle, 0x82, timeout))
                    .unwrap();
                read_active = true;
            }

            if let Some(mut transfer) = async_group.any().unwrap() {
                if transfer.status() == rusb::TransferStatus::Success {
                    rsv_bytes.extend_from_slice(transfer.actual());
                    read_active = false;
                } else {
                    async_group.submit(transfer).unwrap();
                }
            }

            while !rsv_bytes.is_empty() {
                if let Ok((remaining, msg)) = magic::parse(&rsv_bytes) {
                    let removed = rsv_bytes.len() - remaining.len();
                    rsv_bytes = rsv_bytes[removed..].to_vec();
                    info!("parsed {:?}", msg);
                } else {
                    rsv_bytes.clear();
                }
            }
        }
    }
}

fn main() {
    simple_logger::init().unwrap();

    let context = rusb::Context::new().unwrap();
    {
        let mut hubs = get_hubs(&context);
        println!("Found {} bridge(s)", hubs.len());
        if hubs.len() == 0 {
            return;
        }
        println!("Selecting first bridge");
        let hub = hubs.remove(0).open().unwrap();

        trace!("Open hub");
        let mut hub = WyzeHub {
            handle: hub,
            context: &context,
        };

        hub.init();
    }
}
