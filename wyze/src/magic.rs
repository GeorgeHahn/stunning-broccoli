use log::info;

pub fn try_parse(msg: &[u8]) {
    // Remove all leading bytes
    let (msg, _) =
        take_until!(msg, unsafe { std::str::from_utf8_unchecked(&[0x55, 0xAA]) }).unwrap();

    let (msg, _) = take!(msg, 2).unwrap();
    let cmd_type = msg[0];
    let b2 = msg[1];
    let cmd_id = msg[2];

    info!(
        "Found start of msg type: {:?}, b2: {:?}, cmd_id: {:?}",
        cmd_type, b2, cmd_id
    );
}
