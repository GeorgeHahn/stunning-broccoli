use crate::{magic, packet::*};
use std::fmt::Debug;

use log::{info, trace};
use std::io::Write;
use std::time::Duration;

pub struct WyzeHub {
    buf: [u8; 64],
    async_group: rusb::AsyncGroup<'static>,
    handle: rusb::DeviceHandle<'static>,
}

impl WyzeHub {
    pub fn new(handle: rusb::DeviceHandle<'static>, context: &'static rusb::Context) -> WyzeHub {
        info!("Reset");
        handle.reset().unwrap();

        info!("Set active config");
        handle.set_active_configuration(0x00).unwrap();

        if let Ok(result) = handle.kernel_driver_active(0x00) {
            if result {
                info!("Kernel driver active! Detaching");
                handle.detach_kernel_driver(0x00).unwrap();
            }
        }

        info!("Claim interface");
        handle.claim_interface(0x00).unwrap();

        info!("USB HID setup complete");

        WyzeHub {
            buf: [0; 64],
            async_group: rusb::AsyncGroup::new(context),
            handle,
        }
    }

    pub fn init(&mut self) {
        self.send(InquiryPacket);
        let _ = self.raw_read();

        self.send(GetMacPacket);
        let _ = self.raw_read();

        self.send(GetVerPacket);
        let _ = self.raw_read();

        self.send(GetSensorCountPacket);
        let _ = self.raw_read();

        self.send(GetSensorListPacket::create(5));
        let _ = self.raw_read();
        let _ = self.raw_read();
        let _ = self.raw_read();

        self.send(AuthPacket::create_done());

        info!("Hub setup complete");

        loop {
            let _ = self.raw_read();
        }
    }

    fn send<P>(&mut self, packet: P)
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

        self.write_all(&write).unwrap();
    }

    fn raw_read(&mut self) -> Result<(), ()> {
        let timeout = Duration::from_secs(1);

        self.async_group
            .submit(rusb::Transfer::interrupt(
                &self.handle,
                0x82,
                &mut self.buf,
                timeout,
            ))
            .unwrap();

        loop {
            if let Some(mut transfer) = self.async_group.any().unwrap() {
                if transfer.status() == rusb::TransferStatus::Success {
                    let bytes = transfer.actual();
                    info!("read {:?} bytes", bytes.len());
                    let (remaining, msg) = magic::parse(&bytes).unwrap();
                    info!("1: {:?}", msg);
                    if let Ok((_remaining, msg)) = magic::parse(&remaining) {
                        info!("2: {:?}", msg);
                    }
                    return Ok(());
                }
                self.async_group.submit(transfer).unwrap();
            }
        }
    }
}

impl Write for WyzeHub {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.handle
            .write_control(
                0x21,   // rusb_REQUEST_TYPE_CLASS | rusb_RECIPIENT_INTERFACE | rusb_ENDPOINT_OUT
                0x09,   // HID SET_REPORT
                0x02AA, // Report number 0xAA
                0x0000,
                buf,
                std::time::Duration::new(1, 0),
            )
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        // Nothing for us to actually flush because of libUSB arch
        Ok(())
    }
}

// impl Read for WyzeHub<'_> {
//     fn read(&mut self, buf: &mut [u8]) -> Result<usize> {}
// }
