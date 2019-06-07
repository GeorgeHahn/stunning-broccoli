extern crate libusb;

const HUB_VENDOR_ID: u16 = 0x1A86;
const HUB_PRODUCT_ID: u16 = 0xE024;

pub struct WyzeHub<'a> {
    handle: libusb::Device<'a>,
}

impl<'a> WyzeHub<'a> {
    // The constructor will only build a WyzeHub instance if the USB handle
    // corresponds to a valid Wyze Hub
    pub fn new(handle: libusb::Device<'a>) -> Result<WyzeHub<'a>, ()> {
        let device_desc = handle.device_descriptor().map_err(|_| ())?;

        if device_desc.vendor_id() == HUB_VENDOR_ID && device_desc.product_id() == HUB_PRODUCT_ID {
            return Ok(WyzeHub { handle });
        } else {
            return Err(());
        }
    }
}

pub fn get_hubs<'a>(context: &'a libusb::Context) -> Vec<WyzeHub<'a>> {
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

fn main() {
    let context = libusb::Context::new().unwrap();
    let mut hubs = get_hubs(&context);
    println!("Found {} hubs", hubs.len());
    if hubs.len() == 0 {
        return;
    }

    let hub = &mut hubs[0].handle.open().unwrap();

    hub.reset().unwrap();
    hub.set_active_configuration(0x00).unwrap();
    hub.claim_interface(0x0000).unwrap();

    let msg1 = vec![0xAA, 0x55, 0x43, 0x03, 0x27, 0x01, 0x6C];
    println!("Trying to write");
    hub.write_control(
        0x21,
        0x09,
        0x02AA,
        0x0000,
        &msg1,
        std::time::Duration::new(5, 0),
    )
    .unwrap();

    let mut raw_data = [0; 100];

    let len = hub
        .read_bulk(0x82, &mut raw_data, std::time::Duration::new(1, 0))
        .unwrap();

    println!("Read {:?}, data: {:?}", len, &raw_data[..len]);
    for i in 0..len {
        print!("0x{:02X} ", raw_data[i]);
    }
}
