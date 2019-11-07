extern crate log;
extern crate nom;
extern crate rusb;
extern crate simple_logger;

use std::fmt::Debug;

use log::{info, trace};
use std::sync::Arc;
use std::time::Duration;

mod magic;
mod packet;

use packet::*;

const HUB_VENDOR_ID: u16 = 0x1A86;
const HUB_PRODUCT_ID: u16 = 0xE024;

pub struct WyzeHub<'a> {
    device: rusb::Device<'a>,
}

impl<'a> WyzeHub<'a> {
    pub fn get_hubs(context: &'a rusb::Context) -> Vec<WyzeHub<'a>> {
        match context.devices() {
            Ok(devices) => {
                let mut hubs = vec![];
                for device in devices.iter() {
                    match WyzeHub::new(device) {
                        Ok(hub) => hubs.push(hub),
                        Err(_) => (),
                    }
                }
                return hubs;
            }
            Err(_) => return vec![],
        }
    }

    // The constructor will only build a WyzeHub instance if the USB handle
    // corresponds to a valid Wyze Hub
    pub fn new(device: rusb::Device) -> Result<WyzeHub, ()> {
        let device_desc = device.device_descriptor().map_err(|_| ())?;

        if device_desc.vendor_id() == HUB_VENDOR_ID && device_desc.product_id() == HUB_PRODUCT_ID {
            return Ok(WyzeHub { device });
        } else {
            return Err(());
        }
    }

    pub fn open(self) -> OpenWyzeHub<'a> {
        trace!("Open hub");
        let handle = self.device.open().unwrap();
        OpenWyzeHub {
            //device: self.device,
            handle: handle,
            buf: [0; 64],
            rsv_bytes: vec![],
        }
    }
}

pub struct OpenWyzeHub<'a> {
    handle: rusb::DeviceHandle<'a>,
    buf: [u8; 64],
    rsv_bytes: Vec<u8>,
}

impl<'a> OpenWyzeHub<'a> {
    pub fn init(&mut self, context: &'a rusb::Context) {
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
        let _ = self.raw_read(context);
        self.service_bytes();

        self.send(GetMacPacket);
        let _ = self.raw_read(context);
        self.service_bytes();

        self.send(GetVerPacket);
        let _ = self.raw_read(context);
        self.service_bytes();

        self.send(GetSensorCountPacket);
        let _ = self.raw_read(context);
        self.service_bytes();

        self.send(GetSensorListPacket::create(5));
        let _ = self.raw_read(context);
        self.service_bytes();
        let _ = self.raw_read(context);
        self.service_bytes();
        let _ = self.raw_read(context);
        self.service_bytes();

        self.send(AuthPacket::create_done());

        info!("Hub setup complete");

        loop {
            let _ = self.raw_read(context);
            self.service_bytes();
        }
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

    fn raw_read(&mut self, context: &'a rusb::Context) -> Result<(), ()> {
        let mut async_group = rusb::AsyncGroup::new(context);
        let timeout = Duration::from_secs(1);

        async_group
            .submit(rusb::Transfer::interrupt(
                &self.handle,
                0x82,
                &mut self.buf,
                timeout,
            ))
            .unwrap();

        loop {
            if let Some(mut transfer) = async_group.any().unwrap() {
                if transfer.status() == rusb::TransferStatus::Success {
                    self.rsv_bytes.extend_from_slice(transfer.actual());
                    return Ok(());
                }
                async_group.submit(transfer).unwrap();
            }
        }
    }

    fn service_bytes(&mut self) {
        while !self.rsv_bytes.is_empty() {
            if let Ok((remaining, msg)) = magic::parse(&self.rsv_bytes) {
                let removed = self.rsv_bytes.len() - remaining.len();
                self.rsv_bytes = self.rsv_bytes[removed..].to_vec();
                info!("{:?}", msg);
            } else {
                self.rsv_bytes.clear();
            }
        }
    }
}

fn main() {
    simple_logger::init().unwrap();

    let context = rusb::Context::new().unwrap();
    let mut hubs = WyzeHub::get_hubs(&context);
    println!("Found {} bridge(s)", hubs.len());
    if hubs.len() == 0 {
        return;
    }
    println!("Selecting first bridge");
    let hub = hubs.remove(0);
    let mut hub = hub.open();
    hub.init(&context);
}
