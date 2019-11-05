extern crate libusb;
extern crate log;
#[macro_use]
extern crate nom;
extern crate simple_logger;

use std::fmt::Debug;

use log::trace;
use bytes::{Bytes, BytesMut};
use bytes::BufMut;

mod magic;

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
pub enum PacketSyncType {
    Sync,
    Async,
}

pub struct ReceivedPacket<T>
    where T: Packet 
{
    pub lqi: u8,
    pub packet_type: PacketType,
    pub packet: T
}

impl<T> ReceivedPacket<T>
    where T: Packet
{
    pub fn into_inner(self) -> T {
        self.packet
    }
}

pub enum PacketType {
    GetEnr,
    Auth,
    GetMac,
    GetKey,
    Inquiry,
    GetVer,
    GetSensorCount,
    SetRandom,
    StartStopNetwork,
    GetSensorList,
    Event,
    AddSensor,
    Ack,
}

pub trait Packet {
    fn get_packet_type(&self) -> PacketSyncType;
    
    fn get_packet_id(&self) -> u8;
}

pub trait Parseable {
    fn from_bytes(&self, data: Bytes) -> Self;
}

pub trait Packable {
    fn to_bytes(&self) -> Bytes;
}

impl Packable for Packet {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

pub struct EnrPacket;
impl Packet for EnrPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Sync
    }

    fn get_packet_id(&self) -> u8 {
        0x02
    }
}

#[derive(Debug)]
pub struct AuthPacket {
    completion: u8,
}
impl AuthPacket {
    pub fn create_done() -> AuthPacket {
        AuthPacket {
            completion: 0xFF,
        }
    }
    
    pub fn create_blinking() -> AuthPacket {
        AuthPacket {
            completion: 0x00,
        }
    }
}
impl Packet for AuthPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x14
    }
}

impl Packable for AuthPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(2);
        buf.put_u8(self.get_packet_id());
        buf.put_u8(self.completion);
        buf.into()
    }
}

#[derive(Debug)]
pub struct GetMacPacket;
impl Packet for GetMacPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Sync
    }

    fn get_packet_id(&self) -> u8 {
        0x04
    }
}

impl Packable for GetMacPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

#[derive(Debug)]
pub struct GetKeyPacket;
impl Packet for GetKeyPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Sync
    }

    fn get_packet_id(&self) -> u8 {
        0x06
    }
}

#[derive(Debug)]
pub struct InquiryPacket;
impl Packet for InquiryPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Sync
    }

    fn get_packet_id(&self) -> u8 {
        0x27
    }
}

impl Packable for InquiryPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

#[derive(Debug)]
pub struct GetVerPacket;
impl Packet for GetVerPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x16
    }
}

impl Packable for GetVerPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

#[derive(Debug)]
pub struct GetSensorCountPacket;
impl Packet for GetSensorCountPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x2E
    }
}

impl Packable for GetSensorCountPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

#[derive(Debug)]
pub struct SetRandomPacket {
    data: [u8; 16],
}
impl Packet for SetRandomPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x21
    }
}

impl Packable for SetRandomPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(17);
        buf.put_u8(self.get_packet_id());
        buf.put_slice(&self.data);
        buf.into()
    }
}
impl SetRandomPacket {
    pub fn create(data: [u8; 16]) -> SetRandomPacket {
        SetRandomPacket {
            data
        } 
    }
}

#[derive(Debug)]
pub struct StartStopNetworkPacket {
    join_mode: bool,
}
impl Packet for StartStopNetworkPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x1C
    }
}

impl Packable for StartStopNetworkPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(2);
        buf.put_u8(self.get_packet_id());
        buf.put_u8(if self.join_mode { 0x01 } else { 0x00 });
        buf.into()
    }
}
impl StartStopNetworkPacket {
    pub fn create(join_mode: bool) -> StartStopNetworkPacket {
        StartStopNetworkPacket {
            join_mode
        } 
    }
}

#[derive(Debug)]
pub struct GetSensorListPacket {
    count: u8,
}

impl Packet for GetSensorListPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x30
    }
}

impl Packable for GetSensorListPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(2);
        buf.put_u8(self.get_packet_id());
        buf.put_u8(self.count);
        buf.into()
    }
}

impl GetSensorListPacket {
    pub fn create(count: u8) -> GetSensorListPacket {
        GetSensorListPacket {
            count
        } 
    }
}


// 2019-06-24 22:20:25,984 TRACE [wyze] Read 63: [3E, 55, AA, 53, 19, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0E, A2, 37, 37, 37, 42, 31, 39, 36, 32, 01, 01, 00, 51, 04, 5C, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 1, 0, 51, 3D, 4, EE]
// 2019-06-24 22:20:31,836 TRACE [wyze] Read 63: [3E, 55, AA, 53, 19, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0E, A2, 37, 37, 37, 42, 31, 39, 36, 32, 01, 00, 00, 52, 04, 5C, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
#[derive(Debug)]
pub struct SensorEventPacket {
    // preamble, len, id:
    // XX YY 17 35
    // payload:
    // 00 00 01 6A DD 39 43 80 0C A3 <37 37 37 42 31 39 36 32> <01> 10
    // 0  1  2  3  4  5  6  7  8  9   10 11 12 13 14 15 16 17   18  19
    // checksum:
    // 06 5B

    // timestamp ?
    // device id (ASCII) b 10 - b17
    // Device type b 18
    // b 19-21?

    device_id: String,
    device_type: u8,
}
impl Packet for SensorEventPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x35
    }
}

impl Packable for SensorEventPacket {
    fn to_bytes(&self) -> Bytes {
        // This is an incoming message
        unimplemented!()
    }
}


// 2019-06-24 22:20:31,928 TRACE [wyze] Read 63: [21, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// 2019-06-24 22:20:32,016 TRACE [wyze] Read 63: [21, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// 2019-06-24 22:20:32,103 TRACE [wyze] Read 63: [21, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// 2019-06-24 22:21:24,164 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// 2019-06-24 22:21:24,251 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// 2019-06-24 22:21:24,338 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// 2019-06-24 22:21:24,426 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
#[derive(Debug)]
pub struct SensorAlarmPacket {
    // state, battery (% in hex), signal strength
}
impl Packet for SensorAlarmPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x19
    }
}

impl Packable for SensorAlarmPacket {
    fn to_bytes(&self) -> Bytes {
        // This is an incoming message
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct SensorScanPacket {
    // Stuff
}
impl Packet for SensorScanPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x20
    }
}

impl Packable for SensorScanPacket {
    fn to_bytes(&self) -> Bytes {
        // This is an incoming message
        unimplemented!()
    }
}

// 2019-06-24 22:20:57,659 TRACE [wyze] Read 63: [7, 55, AA, 53, 3, 32, 1, 87, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
#[derive(Debug)]
pub struct SensorNotifySyncTimePacket {
    // Stuff
}
impl Packet for SensorNotifySyncTimePacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x32
    }
}

impl Packable for SensorNotifySyncTimePacket {
    fn to_bytes(&self) -> Bytes {
        // This is an incoming message
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct SyncTimeResponsePacket {
    // Stuff
}
impl Packet for SyncTimeResponsePacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x33
    }
}

impl Packable for SyncTimeResponsePacket {
    fn to_bytes(&self) -> Bytes {
        // This is an incoming message
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct AddSensorPacket {
    // TODO: sensor MAC, type, version
}
impl Packet for AddSensorPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x20
    }
}

impl Packable for AddSensorPacket {
    fn to_bytes(&self) -> Bytes {
        // This is an incoming message
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct DeleteSensorCommandPacket {
    // Something?
}
impl Packet for DeleteSensorCommandPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0x25
    }
}

impl Packable for DeleteSensorCommandPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(1);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

#[derive(Debug)]
pub struct AckPacket {
    for_packet_id: u8,
}

impl Packet for AckPacket {
    fn get_packet_type(&self) -> PacketSyncType {
        PacketSyncType::Async
    }

    fn get_packet_id(&self) -> u8 {
        0xFF
    }
}

impl Packable for AckPacket {
    fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(2);
        buf.put_u8(self.for_packet_id);
        buf.put_u8(self.get_packet_id());
        buf.into()
    }
}

impl<'a> OpenWyzeHub<'a> {
    pub fn init(&mut self) {
        trace!("Reset");
        self.handle.reset().unwrap();

        trace!("Set active config");
        self.handle.set_active_configuration(0x00).unwrap();

        trace!("Claim interface");
        self.handle.claim_interface(0x0000).unwrap();

        trace!("USB HID setup complete");

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

        trace!("Hub setup complete");

        loop {
            let _ = self.raw_read();
        }
    }

    fn send<P>(&self, packet: P)
        where P: Packet + Packable + Debug
    {
        trace!("Sending packet {:?}", packet);
        let mut write: Vec<u8> = Vec::new();
        let data = packet.to_bytes();

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
        let ck: u16 = write.iter().fold(0u16, |acc, x| acc.wrapping_add(*x as u16));
        let ck_bytes: &[u8] = &[(ck >> 8 & 0xFF) as u8, (ck & 0xFF) as u8];
        write.extend(ck_bytes);

        self.raw_write(write);
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
                magic::try_parse(rsp);
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
