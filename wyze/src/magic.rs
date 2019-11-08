use nom::bytes::complete::{take, take_until};
use nom::number::complete::be_u8;
use nom::IResult;

#[derive(Debug)]
pub enum PacketType {
    Async = 0x53,
    Sync = 0x43,
}

#[derive(Debug)]
pub struct RawMessage {
    cmd_type: PacketType,
    pub cmd_id: u8,
    pub payload: Vec<u8>,
}

pub fn parse(msg: &[u8]) -> IResult<&[u8], RawMessage> {
    // Remove all leading bytes
    let leader = [0x55, 0xAA];
    let (msg, _) = take_until(&leader[..])(msg)?;
    let (msg, _) = take(leader.len())(msg)?;
    let (msg, cmd_type) = be_u8(msg)?;
    let (msg, length) = be_u8(msg)?;
    let (msg, cmd_id) = be_u8(msg)?;

    let (msg, payload, _chksum) = if cmd_id == 0xFF {
        let (msg, payload) = take(0usize)(msg)?;
        let (msg, chksum) = take(2usize)(msg)?;
        (msg, payload, chksum)
    } else {
        let (msg, payload) = take(length as usize - 3)(msg)?; // 3 -> 1:cmd + 2:chksum
        let (msg, chksum) = take(2usize)(msg)?;
        (msg, payload, chksum)
    };

    let cmd_type = if cmd_type == 0x53 {
        PacketType::Async
    } else {
        PacketType::Sync
    };

    Ok((
        msg,
        RawMessage {
            cmd_type,
            cmd_id,
            payload: payload.to_vec(),
        },
    ))
}
