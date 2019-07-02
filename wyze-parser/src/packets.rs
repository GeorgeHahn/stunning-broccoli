#[derive(Debug, FromPrimitive)]
pub enum PacketSyncType {
    Async = 0x53,
    Sync = 0x43,
}

#[derive(Debug)]
pub enum MessageType {
    Command,
    Response,
    Ack,
}

#[derive(Debug)]
pub enum PacketSource {
    Bridge, // 55 AA
    Host,   // AA 55
}

pub trait Packet {
    type BaseType;
    const CMD_ID: u8;
    const RSP_ID: u8;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()>;
    fn pack(&self) -> Vec<u8>;
    fn message_type(&self) -> MessageType;
}

macro_rules! PacketPayloadBuilder {
    ($($x:ident),+) => {
#[derive(PartialEq, Debug, Clone)]
#[allow(dead_code)]
pub enum PacketPayload {
    $($x($x)),+
}

impl PacketPayload {
    pub fn parse<'a, 'b>(input: &'b[u8], id: u8, ack: bool) -> Result<PacketPayload, ()> {
        match id {
            $($x::CMD_ID => {
                let payload = if ack {
                    $x::parse(input, MessageType::Ack)?
                } else {
                    $x::parse(input, MessageType::Command)?
                };
                return Ok(PacketPayload::$x(payload));
            },
            $x::RSP_ID => {
                let payload = if ack {
                    $x::parse(input, MessageType::Ack)?
                } else {
                    $x::parse(input, MessageType::Response)?
                };
                return Ok(PacketPayload::$x(payload));
            }),+
            _ => Err(()),
        }
    }

    // pub fn pack(self) -> Vec<u8> {
    //     match self {
    //         $(PacketPayload::$x(payload) => {return payload.pack()}),+
    //     }
    // }
}
}}

PacketPayloadBuilder!(
    InquiryPacket,
    MacPacket,
    VersionPacket,
    SensorCountPacket,
    SensorListPacket,
    AuthPacket
);

#[derive(Debug)]
pub struct PacketHandle {
    payload: PacketPayload,
    sync_type: PacketSyncType,
}

impl PacketHandle {
    pub fn parse(
        input: &[u8],
        id: u8,
        ack: bool,
        sync_type: PacketSyncType,
    ) -> Result<PacketHandle, ()> {
        let payload = PacketPayload::parse(input, id, ack).map_err(|_| ())?;
        Ok(PacketHandle { payload, sync_type })
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum InquiryPacket {
    Command,
    Response { value: u8 },
    Ack,
}

impl Packet for InquiryPacket {
    type BaseType = Self;

    const CMD_ID: u8 = 0x27;
    const RSP_ID: u8 = 0x28;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()> {
        match msg_type {
            MessageType::Command => Ok(InquiryPacket::Command),
            MessageType::Response => {
                let value = input.first().ok_or(())?;
                Ok(InquiryPacket::Response { value: *value })
            }
            MessageType::Ack => Ok(InquiryPacket::Ack),
        }
    }

    fn pack(&self) -> Vec<u8> {
        match self {
            InquiryPacket::Command => vec![],
            InquiryPacket::Response { value } => vec![*value],
            InquiryPacket::Ack => vec![],
        }
    }

    fn message_type(&self) -> MessageType {
        match self {
            InquiryPacket::Command => MessageType::Command,
            InquiryPacket::Response { value: _ } => MessageType::Response,
            InquiryPacket::Ack => MessageType::Ack,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum MacPacket {
    Command,
    Response { mac: String },
    Ack,
}

impl Packet for MacPacket {
    type BaseType = Self;

    const CMD_ID: u8 = 0x04;
    const RSP_ID: u8 = 0x05;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()> {
        match msg_type {
            MessageType::Command => Ok(MacPacket::Command),
            MessageType::Response => {
                let mac = std::str::from_utf8(input).map_err(|_| ())?;
                Ok(MacPacket::Response {
                    mac: mac.to_string(),
                })
            }
            MessageType::Ack => Ok(MacPacket::Ack),
        }
    }

    fn pack(&self) -> Vec<u8> {
        match self {
            MacPacket::Command => vec![],
            MacPacket::Response { mac } => mac.as_bytes().to_vec(),
            MacPacket::Ack => vec![],
        }
    }

    fn message_type(&self) -> MessageType {
        match self {
            MacPacket::Command => MessageType::Command,
            MacPacket::Response { mac: _ } => MessageType::Response,
            MacPacket::Ack => MessageType::Ack,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum VersionPacket {
    Command,
    Response {
        fw_version: String,
        hw_version: String,
        hw_type: String,
        magic: String,
    },
    Ack,
}

impl Packet for VersionPacket {
    type BaseType = Self;

    const CMD_ID: u8 = 0x16;
    const RSP_ID: u8 = 0x17;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()> {
        match msg_type {
            MessageType::Command => Ok(VersionPacket::Command),
            MessageType::Response => {
                let mac = std::str::from_utf8(input).map_err(|_| ())?;
                let mac = mac.split(" ").collect::<Vec<_>>();
                Ok(VersionPacket::Response {
                    fw_version: mac[0].to_string(),
                    hw_version: mac[1].to_string(),
                    hw_type: mac[2].to_string(),
                    magic: mac[3].to_string(),
                })
            }
            MessageType::Ack => Ok(VersionPacket::Ack),
        }
    }

    fn pack(&self) -> Vec<u8> {
        match self {
            VersionPacket::Command => vec![],
            VersionPacket::Response {
                fw_version: _,
                hw_version: _,
                hw_type: _,
                magic: _,
            } => vec![],
            VersionPacket::Ack => vec![],
        }
    }

    fn message_type(&self) -> MessageType {
        match self {
            VersionPacket::Command => MessageType::Command,
            VersionPacket::Response {
                fw_version: _,
                hw_version: _,
                hw_type: _,
                magic: _,
            } => MessageType::Response,
            VersionPacket::Ack => MessageType::Ack,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum SensorCountPacket {
    Command,
    Response { count: u8 },
    Ack,
}

impl Packet for SensorCountPacket {
    type BaseType = Self;

    const CMD_ID: u8 = 0x2E;
    const RSP_ID: u8 = 0x2F;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()> {
        match msg_type {
            MessageType::Command => Ok(SensorCountPacket::Command),
            MessageType::Response => {
                let count = input.first().ok_or(())?;
                Ok(SensorCountPacket::Response { count: *count })
            }
            MessageType::Ack => Ok(SensorCountPacket::Ack),
        }
    }

    fn pack(&self) -> Vec<u8> {
        match self {
            SensorCountPacket::Command => vec![],
            SensorCountPacket::Response { count } => vec![*count],
            SensorCountPacket::Ack => vec![],
        }
    }

    fn message_type(&self) -> MessageType {
        match self {
            SensorCountPacket::Command => MessageType::Command,
            SensorCountPacket::Response { count: _ } => MessageType::Response,
            SensorCountPacket::Ack => MessageType::Ack,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum SensorListPacket {
    Command { count: u8 },
    Response { mac: String },
    Ack,
}

impl Packet for SensorListPacket {
    type BaseType = Self;

    const CMD_ID: u8 = 0x30;
    const RSP_ID: u8 = 0x31;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()> {
        match msg_type {
            MessageType::Command => {
                let count = input.first().ok_or(())?;
                Ok(SensorListPacket::Command { count: *count })
            }
            MessageType::Response => {
                let mac = std::str::from_utf8(input).map_err(|_| ())?;
                Ok(SensorListPacket::Response {
                    mac: mac.to_string(),
                })
            }
            MessageType::Ack => Ok(SensorListPacket::Ack),
        }
    }

    fn pack(&self) -> Vec<u8> {
        match self {
            SensorListPacket::Command { count } => vec![*count],
            SensorListPacket::Response { mac } => mac.as_bytes().to_vec(),
            SensorListPacket::Ack => vec![],
        }
    }

    fn message_type(&self) -> MessageType {
        match self {
            SensorListPacket::Command { count: _ } => MessageType::Command,
            SensorListPacket::Response { mac: _ } => MessageType::Response,
            SensorListPacket::Ack => MessageType::Ack,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum AuthPacket {
    Command { completion: u8 },
    Response,
    Ack,
}

impl Packet for AuthPacket {
    type BaseType = Self;

    const CMD_ID: u8 = 0x14;
    const RSP_ID: u8 = 0x15;

    fn parse(input: &[u8], msg_type: MessageType) -> Result<Self::BaseType, ()> {
        match msg_type {
            MessageType::Command => {
                let completion = input.first().ok_or(())?;
                Ok(AuthPacket::Command {
                    completion: *completion,
                })
            }
            MessageType::Response => Ok(AuthPacket::Response),
            MessageType::Ack => Ok(AuthPacket::Ack),
        }
    }

    fn pack(&self) -> Vec<u8> {
        match self {
            AuthPacket::Command { completion } => vec![*completion],
            AuthPacket::Response => vec![],
            AuthPacket::Ack => vec![],
        }
    }

    fn message_type(&self) -> MessageType {
        match self {
            AuthPacket::Command { completion: _ } => MessageType::Command,
            AuthPacket::Response => MessageType::Response,
            AuthPacket::Ack => MessageType::Ack,
        }
    }
}

// pub enum EnrPacket {
//     Command,
//     Response,
//     Ack,
// }

// impl Packet for EnrPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Sync
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x02
//     }
// }

// #[derive(Debug)]
// pub struct AuthPacket {
//     completion: u8,
// }
// impl AuthPacket {
//     pub fn create_done() -> AuthPacket {
//         AuthPacket {
//             completion: 0xFF,
//         }
//     }

//     pub fn create_blinking() -> AuthPacket {
//         AuthPacket {
//             completion: 0x00,
//         }
//     }
// }
// impl Packet for AuthPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x14
//     }
// }

// impl Packable for AuthPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(2);
//         buf.put_u8(self.get_packet_id());
//         buf.put_u8(self.completion);
//         buf.into()
//     }
// }

// #[derive(Debug)]
// pub struct GetKeyPacket;
// impl Packet for GetKeyPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Sync
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x06
//     }
// }

// #[derive(Debug)]
// pub struct GetVerPacket;
// impl Packet for GetVerPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x16
//     }
// }

// impl Packable for GetVerPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(1);
//         buf.put_u8(self.get_packet_id());
//         buf.into()
//     }
// }

// #[derive(Debug)]
// pub struct GetSensorCountPacket;
// impl Packet for GetSensorCountPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x2E
//     }
// }

// impl Packable for GetSensorCountPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(1);
//         buf.put_u8(self.get_packet_id());
//         buf.into()
//     }
// }

// #[derive(Debug)]
// pub struct SetRandomPacket {
//     data: [u8; 16],
// }
// impl Packet for SetRandomPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x21
//     }
// }

// impl Packable for SetRandomPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(17);
//         buf.put_u8(self.get_packet_id());
//         buf.put_slice(&self.data);
//         buf.into()
//     }
// }
// impl SetRandomPacket {
//     pub fn create(data: [u8; 16]) -> SetRandomPacket {
//         SetRandomPacket {
//             data
//         }
//     }
// }

// #[derive(Debug)]
// pub struct StartStopNetworkPacket {
//     join_mode: bool,
// }
// impl Packet for StartStopNetworkPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x1C
//     }
// }

// impl Packable for StartStopNetworkPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(2);
//         buf.put_u8(self.get_packet_id());
//         buf.put_u8(if self.join_mode { 0x01 } else { 0x00 });
//         buf.into()
//     }
// }
// impl StartStopNetworkPacket {
//     pub fn create(join_mode: bool) -> StartStopNetworkPacket {
//         StartStopNetworkPacket {
//             join_mode
//         }
//     }
// }

// #[derive(Debug)]
// pub struct GetSensorListPacket {
//     count: u8,
// }

// impl Packet for GetSensorListPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x30
//     }
// }

// impl Packable for GetSensorListPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(2);
//         buf.put_u8(self.get_packet_id());
//         buf.put_u8(self.count);
//         buf.into()
//     }
// }

// impl GetSensorListPacket {
//     pub fn create(count: u8) -> GetSensorListPacket {
//         GetSensorListPacket {
//             count
//         }
//     }
// }

// // 2019-06-24 22:20:25,984 TRACE [wyze] Read 63: [3E, 55, AA, 53, 19, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0E, A2, 37, 37, 37, 42, 31, 39, 36, 32, 01, 01, 00, 51, 04, 5C, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 1, 0, 51, 3D, 4, EE]
// // 2019-06-24 22:20:31,836 TRACE [wyze] Read 63: [3E, 55, AA, 53, 19, 35, 0, 0, 0, 0, 0, 0, 0, 0, 0E, A2, 37, 37, 37, 42, 31, 39, 36, 32, 01, 00, 00, 52, 04, 5C, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// #[derive(Debug)]
// pub struct SensorEventPacket {
//     // preamble, len, id:
//     // XX YY 17 35
//     // payload:
//     // 00 00 01 6A DD 39 43 80 0C A3 <37 37 37 42 31 39 36 32> <01> 10
//     // 0  1  2  3  4  5  6  7  8  9   10 11 12 13 14 15 16 17   18  19
//     // checksum:
//     // 06 5B

//     // timestamp ?
//     // device id (ASCII) b 10 - b17
//     // Device type b 18
//     // b 19-21?

//     device_id: String,
//     device_type: u8,
// }
// impl Packet for SensorEventPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x35
//     }
// }

// impl Packable for SensorEventPacket {
//     fn to_bytes(&self) -> Bytes {
//         // This is an incoming message
//         unimplemented!()
//     }
// }

// // 2019-06-24 22:20:31,928 TRACE [wyze] Read 63: [21, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// // 2019-06-24 22:20:32,016 TRACE [wyze] Read 63: [21, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// // 2019-06-24 22:20:32,103 TRACE [wyze] Read 63: [21, 55, AA, 53, 1D, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// // 2019-06-24 22:21:24,164 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// // 2019-06-24 22:21:24,251 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// // 2019-06-24 22:21:24,338 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// // 2019-06-24 22:21:24,426 TRACE [wyze] Read 63: [27, 55, AA, 53, 23, 19, 0, 0, 0, 0, 0, 0, 0, 0, AB, 37, 37, 37, 41, 43, 32, 36, 30, 2, 1, 5, 3, 5, 3, 7, 5, 0, 7, 5, 4, 0, 40, 0, 4, 69, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// #[derive(Debug)]
// pub struct SensorAlarmPacket {
//     // state, battery (% in hex), signal strength
// }
// impl Packet for SensorAlarmPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x19
//     }
// }

// impl Packable for SensorAlarmPacket {
//     fn to_bytes(&self) -> Bytes {
//         // This is an incoming message
//         unimplemented!()
//     }
// }

// #[derive(Debug)]
// pub struct SensorScanPacket {
//     // Stuff
// }
// impl Packet for SensorScanPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x20
//     }
// }

// impl Packable for SensorScanPacket {
//     fn to_bytes(&self) -> Bytes {
//         // This is an incoming message
//         unimplemented!()
//     }
// }

// // 2019-06-24 22:20:57,659 TRACE [wyze] Read 63: [7, 55, AA, 53, 3, 32, 1, 87, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5, 19, 0, 0, 0, 0, 0, 0, 0, 0, A2, 37, 37, 37, 42, 31, 39, 36, 32, 1, 1A, 60, 0, 1, 0, 0, 52, 44, 4, F5]
// #[derive(Debug)]
// pub struct SensorNotifySyncTimePacket {
//     // Stuff
// }
// impl Packet for SensorNotifySyncTimePacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x32
//     }
// }

// impl Packable for SensorNotifySyncTimePacket {
//     fn to_bytes(&self) -> Bytes {
//         // This is an incoming message
//         unimplemented!()
//     }
// }

// #[derive(Debug)]
// pub struct SyncTimeResponsePacket {
//     // Stuff
// }
// impl Packet for SyncTimeResponsePacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x33
//     }
// }

// impl Packable for SyncTimeResponsePacket {
//     fn to_bytes(&self) -> Bytes {
//         // This is an incoming message
//         unimplemented!()
//     }
// }

// #[derive(Debug)]
// pub struct AddSensorPacket {
//     // TODO: sensor MAC, type, version
// }
// impl Packet for AddSensorPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x20
//     }
// }

// impl Packable for AddSensorPacket {
//     fn to_bytes(&self) -> Bytes {
//         // This is an incoming message
//         unimplemented!()
//     }
// }

// #[derive(Debug)]
// pub struct DeleteSensorCommandPacket {
//     // Something?
// }
// impl Packet for DeleteSensorCommandPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0x25
//     }
// }

// impl Packable for DeleteSensorCommandPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(1);
//         buf.put_u8(self.get_packet_id());
//         buf.into()
//     }
// }

// #[derive(Debug)]
// pub struct AckPacket {
//     for_packet_id: u8,
// }

// impl Packet for AckPacket {
//     fn get_packet_type(&self) -> PacketSyncType {
//         PacketSyncType::Async
//     }

//     fn get_packet_id(&self) -> u8 {
//         0xFF
//     }
// }

// impl Packable for AckPacket {
//     fn to_bytes(&self) -> Bytes {
//         let mut buf = BytesMut::with_capacity(2);
//         buf.put_u8(self.for_packet_id);
//         buf.put_u8(self.get_packet_id());
//         buf.into()
//     }
// }
