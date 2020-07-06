pub type HeaderBlock = [u8; 8];
pub type ByteBlock = [u8; 9];

pub fn validate_header(block: &HeaderBlock) -> bool {
    return block == b"MAX7456\n";
}

pub fn char_block_to_byte(block: &ByteBlock) -> Option<u8> {
    if block[8] != b'\n' {
        return None;
    }
    let mut byte = 0u8;
    for i in 0..8 {
        byte <<= 1;
        byte |= (block[i] - b'0') & 1;
    }
    Some(byte)
}
