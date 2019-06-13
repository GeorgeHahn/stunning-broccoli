extern crate libusb;
extern crate log;
extern crate simple_logger;

use log::{error, trace};

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
        OpenWyzeHub {
            _device: self.device,
            handle: handle,
            buf: [0; 64],
        }
    }
}

pub struct OpenWyzeHub<'a> {
    _device: libusb::Device<'a>,
    handle: libusb::DeviceHandle<'a>,
    buf: [u8; 64],
}

#[derive(Debug)]
pub enum PacketType {
    Sync,
    Async,
}

#[derive(Debug)]
pub struct Command {
    pub pt: PacketType,
    pub value: u8,
    pub payload: Option<Vec<u8>>,
}

impl Command {
    pub fn get_enr() -> Command {
        error!("get_enr() payload unimplemented");
        Command {
            pt: PacketType::Sync,
            value: 0x02,
            payload: None,
        }
    }

    pub fn get_mac() -> Command {
        Command {
            pt: PacketType::Sync,
            value: 0x04,
            payload: None,
        }
    }

    pub fn get_key() -> Command {
        Command {
            pt: PacketType::Sync,
            value: 0x06,
            payload: None,
        }
    }

    pub fn inquiry() -> Command {
        Command {
            pt: PacketType::Sync,
            value: 0x27,
            payload: None,
        }
    }

    pub fn auth_blinking() -> Command {
        Command {
            pt: PacketType::Async,
            value: 0x14,
            payload: Some(vec![0x00]), // any value 0x00-0xFE
        }
    }

    pub fn auth_done() -> Command {
        Command {
            pt: PacketType::Async,
            value: 0x14,
            payload: Some(vec![0xFF]),
        }
    }

    pub fn get_ver() -> Command {
        Command {
            pt: PacketType::Async,
            value: 0x16,
            payload: None,
        }
    }

    pub fn get_sensor_count() -> Command {
        Command {
            pt: PacketType::Async,
            value: 0x2E,
            payload: None,
        }
    }

    pub fn start_stop_network(joinmode: bool) -> Command {
        Command {
            pt: PacketType::Async,
            value: 0x1C,
            payload: Some(vec![if joinmode { 0x01 } else { 0x00 }]),
        }
    }

    pub fn set_random() -> Command {
        error!("set_random() payload unimplemented");
        Command {
            pt: PacketType::Async,
            value: 0x21,
            payload: None,
        }
    }

    pub fn get_sensor_list(count: u8) -> Command {
        Command {
            pt: PacketType::Async,
            value: 0x30,
            payload: Some(vec![count]),
        }
    }
}

impl<'a> OpenWyzeHub<'a> {
    pub fn init(&mut self) {
        trace!("Reset");
        self.handle.reset().unwrap();

        trace!("Claim interface");
        self.handle.claim_interface(0x0000).unwrap();

        trace!("USB HID setup complete");

        self.send(Command::inquiry());

        // self.send_get_enr();

        self.send(Command::get_mac());

        // self.send_get_key();

        self.send(Command::get_ver());

        self.send(Command::get_sensor_count());

        self.send(Command::get_sensor_list(5));
        let _ = self.raw_read();
        let _ = self.raw_read();
        let _ = self.raw_read();
        let _ = self.raw_read();
        let _ = self.raw_read();

        self.send(Command::auth_done());

        trace!("Hub setup complete");
    }

    fn send(&mut self, cmd: Command) {
        if let Some(p) = cmd.payload {
            self.write_packet(cmd.pt, cmd.value, p);
        } else {
            self.write_packet(cmd.pt, cmd.value, vec![]);
        }
        let _ = self.raw_read();
        // TODO: Validate and return response (sync) / ack (async)?
    }

    fn write_packet(&self, pt: PacketType, cmd: u8, data: Vec<u8>) {
        trace!("Sending {:?} packet 0x{:x}", pt, cmd);
        let mut packet: Vec<u8> = Vec::new();

        // Direction
        packet.extend(&[0xAA, 0x55]);

        // Type
        match pt {
            PacketType::Sync => packet.push(0x43),
            PacketType::Async => packet.push(0x53),
        }

        // Length
        packet.push(data.len() as u8 + 3);

        // command
        packet.push(cmd);

        // payload
        packet.extend(data);

        // checksum
        let ck: u16 = packet.iter().fold(0u16, |acc, x| acc + (*x as u16));
        let ck_bytes: &[u8] = &[(ck >> 8 & 0xFF) as u8, (ck & 0xFF) as u8];
        packet.extend(ck_bytes);

        self.raw_write(packet);
    }

    fn raw_write(&self, data: Vec<u8>) {
        trace!("Sending data {:x?}", &data);

        self.handle
            .write_control(
                0x21,   // LIBUSB_REQUEST_TYPE_CLASS | LIBUSB_RECIPIENT_INTERFACE | LIBUSB_ENDPOINT_OUT
                0x09,   // HID SET_REPORT
                0x02AA, // Report number 0xAA
                0x0000,
                &data,
                std::time::Duration::new(1, 0),
            )
            .unwrap();
    }

    fn raw_read(&mut self) -> Result<&[u8], ()> {
        let rsp = self
            .handle
            .read_interrupt(0x82, &mut self.buf, std::time::Duration::new(1, 0));

        return match rsp {
            Ok(len) => {
                let rsp = &self.buf[..len];
                trace!("Read {:?}: {:X?}", rsp.len(), &rsp);
                Ok(rsp)
            }
            Err(_) => Err(()),
        };
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
