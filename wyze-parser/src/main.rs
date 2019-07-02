extern crate nom;
extern crate num;
#[macro_use]
extern crate num_derive;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take;
use nom::multi::many_till;
use nom::number::complete::{be_u16, be_u8};
use nom::IResult;

use num::FromPrimitive;

mod packets;

// inquiry command
//const MSG: &[u8] = &[0xAA, 0x55, 0x43, 0x3, 0x27, 0x01, 0x6c];

// inquiry response
//const MSG: &[u8] = &[0x55, 0xAA, 0x43, 0x4, 0x28, 0x1, 0x01, 0x6F];

// get mac command
//const MSG: &[u8] = &[0xAA, 0x55, 0x43, 0x3, 0x4, 0x01, 0x49];

// get mac response
// const MSG: &[u8] = &[
//     0x55, 0xAA, 0x43, 0xB, 0x5, 0x37, 0x37, 0x37, 0x41, 0x46, 0x39, 0x42, 0x46, 0x3, 0x3F,
// ];

// get version command
// const MSG: &[u8] = &[0xAA, 0x55, 0x53, 0x03, 0x16, 0x1, 0x6b];

// get version response
// const MSG: &[u8] = &[
//     0x55, 0xAA, 0x53, 0x1C, 0x17, 0x30, 0x2E, 0x30, 0x2E, 0x30, 0x2E, 0x33, 0x30, 0x20, 0x56, 0x31,
//     0x2E, 0x34, 0x20, 0x44, 0x6F, 0x6E, 0x67, 0x6C, 0x65, 0x20, 0x55, 0x44, 0x33, 0x55, 0x7, 0xC5,
// ];

// get sensor count command
// const MSG: &[u8] = &[0xAA, 0x55, 0x53, 0x3, 0x2e, 0x1, 0x83];

// get sensor count ack
// const MSG: &[u8] = &[0x55, 0xAA, 0x53, 0x2E, 0xFF, 0x2, 0x7F];

// get sensor count response
// const MSG: &[u8] = &[0x55, 0xAA, 0x53, 0x4, 0x2F, 0x2, 0x1, 0x87];

// get sensor list command
// const MSG: &[u8] = &[0xAA, 0x55, 0x53, 0x4, 0x30, 0x5, 0x1, 0x8b];

// get sensor list response
// const MSG: &[u8] = &[
//     0x55, 0xAA, 0x53, 0xB, 0x31, 0x37, 0x37, 0x37, 0x42, 0x31, 0x39, 0x36, 0x32, 0x3, 0x47,
// ];

// auth command
// const MSG: &[u8] = &[0xAA, 0x55, 0x53, 0x4, 0x14, 0xff, 0x2, 0x69];

// auth command
const MSG: &[u8] = &[0x55, 0xAA, 0x53, 0x3, 0x15, 0x1, 0x6A];

fn find_msg(input: &[u8]) -> IResult<&[u8], packets::PacketHandle> {
    let (input, (_, preamble)) = many_till(
        take(1 as usize),
        alt((tag([0x55, 0xAA]), tag([0xAA, 0x55]))),
    )(input)?;
    let _source = if preamble[0] == 0x55 {
        packets::PacketSource::Bridge
    } else {
        packets::PacketSource::Host
    };
    let (remaining, type_raw) = take(1 as usize)(input)?;
    let sync_type = packets::PacketSyncType::from_u8(type_raw[0])
        .ok_or(nom::Err::Failure((remaining, nom::error::ErrorKind::IsNot)))?;
    let (remaining, length_or_id) = be_u8(remaining)?;
    let (remaining, ack_or_id) = be_u8(remaining)?;
    let length;
    let id;
    let ack;

    if ack_or_id == 0xFF {
        ack = true;
        id = length_or_id;
        length = 3;
    } else {
        ack = false;
        id = ack_or_id;
        length = length_or_id;
    }

    if length < 2 {
        return Err(nom::Err::Failure((remaining, nom::error::ErrorKind::IsNot)));
    }

    let (remaining, payload) = take(length - 3)(remaining)?;
    let (remaining, chksum_msg) = be_u16(remaining)?;

    let mut chksum_calc: u16 = 0xFF; // Start at 0x00FF to account for the preamble that we dropped earlier
    for i in 0..(length) {
        chksum_calc = chksum_calc.wrapping_add(input[i as usize] as u16);
    }

    if chksum_calc != chksum_msg {
        println!(
            "Got msg chksum: {:04X?}, calced: {:04X?}",
            chksum_msg, chksum_calc
        );
        return Err(nom::Err::Failure((remaining, nom::error::ErrorKind::IsNot)));
    }

    let ph = packets::PacketHandle::parse(payload, id, ack, sync_type).expect("Failed to parse");

    // TODO: Return something actually useful from the parsing
    Ok((remaining, ph))
}

fn main() {
    println!("{:02X?}", find_msg(MSG));
}
