extern crate libusb;
extern crate log;
extern crate simple_logger;

use log::{trace};

const HUB_VENDOR_ID: u16 = 0x1A86;
const HUB_PRODUCT_ID: u16 = 0xE024;

pub struct WyzeHub<'a> {
    device: libusb::Device<'a>,
}

impl<'a> WyzeHub<'a> {
    pub fn get_hubs(context: &'a libusb::Context) -> Vec<WyzeHub<'a>> {
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
    pub fn new(device: libusb::Device) -> Result<WyzeHub, ()> {
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
        OpenWyzeHub { _device: self.device, handle: handle, buf: [0; 1024] }
    }
}

pub struct OpenWyzeHub<'a> {
    _device: libusb::Device<'a>,
    handle: libusb::DeviceHandle<'a>,
    buf: [u8; 1024],
}

impl<'a> OpenWyzeHub<'a> {
    pub fn init(&mut self) {
        trace!("Reset bridge");
        self.handle.reset().unwrap();

        trace!("Set active config");
        self.handle.set_active_configuration(0x01).unwrap();

        trace!("Claim interface");
        self.handle.claim_interface(0x0000).unwrap();

        let msg1 = [0xAA, 0x55, 0x43, 0x03, 0x27, 0x01, 0x6C];
        self.raw_write(&msg1);

        let data = self.raw_read();
        trace!("Read {:?}, data: {:X?}", data.len(), &data);

        let msg2 = [0xAA, 0x55, 0x43, 0x13, 0x02, 0x74, 0x34, 0x6C, 0x67, 0x4C, 0x53, 0x70, 0x33, 0x73, 0x33, 0x39, 0x39, 0x79, 0x4E, 0x75, 0x4A, 0x06, 0xB2, 0x17];
        self.raw_write(&msg2);

        let data = self.raw_read();
        trace!("Read {:?}, data: {:X?}", data.len(), &data);
    }

    fn raw_write(&self, data: &[u8]) {
        trace!("Write data");
        self.handle.write_control(
            0x21,
            0x09,
            0x02AA,
            0x0000,
            &data,
            std::time::Duration::new(1, 0),
        )
        .unwrap();
    }

    fn raw_read(&mut self) -> &[u8] {
        trace!("Read data");
        let len = self.handle
            .read_interrupt(0x82, &mut self.buf, std::time::Duration::new(1, 0))
            .unwrap();

        return &self.buf[..len];
    }
}


fn main() {
    simple_logger::init().unwrap();

    let context = libusb::Context::new().unwrap();
    let mut hubs = WyzeHub::get_hubs(&context);
    println!("Found {} bridge(s)", hubs.len());
    if hubs.len() == 0 {
        return;
    }
    println!("Selecting first bridge");
    let hub = hubs.remove(0);
    let mut hub = hub.open();
    hub.init();
}
