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

#[derive(Debug)]
pub enum PacketSource {
    Bridge, // 55 AA
    Host,   // AA 55
}

#[derive(Debug, FromPrimitive)]
pub enum PacketType {
    Async = 0x53,
    Sync = 0x43,
}

// const MSG: &[u8] = &[
//     0x55, 0xAA, 0x43, 0xB, 0x5, 0x37, 0x37, 0x37, 0x41, 0x46, 0x39, 0x42, 0x46, 0x3, 0x3F, 0x5,
//     0x37, 0x37, 0x37, 0x41, 0x46, 0x39, 0x42, 0x46, 0x3, 0x3F,
// ];

// const MSG: &[u8] = &[0x55, 0xaa, 0x53, 0x16, 0xff, 0x02, 0x67];

const MSG: &[u8] = &[0x55, 0xAA, 0x53, 0x1C, 0x17, 0x30, 0x2E, 0x30, 0x2E, 0x30, 0x2E, 0x33, 0x30, 0x20, 0x56, 0x31, 0x2E, 0x34, 0x20, 0x44, 0x6F, 0x6E, 0x67, 0x6C, 0x65, 0x20, 0x55, 0x44, 0x33, 0x55, 0x7, 0xC5, 0x0, 0x0, 0x0, 0xA2, 0x37, 0x37, 0x37, 0x41, 0x43, 0x32, 0x36, 0x30, 0x2, 0x14, 0x63, 0x0, 0x1, 0x1, 0x2, 0xA3, 0x33, 0x5, 0x3E];

fn find_msg(input: &[u8]) -> IResult<&[u8], PacketSource> {
    let (input, (_, preamble)) = many_till(
        take(1 as usize),
        alt((tag([0x55, 0xAA]), tag([0xAA, 0x55]))),
    )(input)?;
    let source = if preamble[0] == 0x55 {
        PacketSource::Bridge
    } else {
        PacketSource::Host
    };
    let (remaining, type_raw) = take(1 as usize)(input)?;
    let _msg_type = PacketType::from_u8(type_raw[0])
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
    println!("id: {:02X}, ack: {:?}, payload: {:02X?}", id, ack, payload);

    // TODO: Return something actually useful from the parsing
    Ok((remaining, source))
}

fn main() {
    println!("{:02X?}", find_msg(MSG));
}
