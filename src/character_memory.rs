use super::registers::{CharacterMemoryMode, Registers};

pub const CHAR_DATA_SIZE: usize = 64;
pub const STORE_CHAR_BUFFER_SIZE: usize = 2 + CHAR_DATA_SIZE * 4 + 2;

pub type CharData = [u8; CHAR_DATA_SIZE];

pub fn build_store_char_operation(data: &CharData, index: u8, output: &mut [u8]) -> bool {
    if output.len() < STORE_CHAR_BUFFER_SIZE {
        return false;
    }
    output[0] = Registers::CharacterMemoryAddressHigh as u8;
    output[1] = index;
    let mut chunks = output[2..].chunks_mut(4);
    for (i, &ch) in data.iter().enumerate() {
        let chunk = chunks.next().unwrap();
        chunk[0] = Registers::CharacterMemoryAddressLow as u8;
        chunk[1] = i as u8;
        chunk[2] = Registers::CharacterMemoryDataIn as u8;
        chunk[3] = ch;
    }
    output[2 + CHAR_DATA_SIZE * 4] = Registers::CharacterMemoryMode as u8;
    output[2 + CHAR_DATA_SIZE * 4 + 1] = CharacterMemoryMode::WriteToNVM as u8;
    return true;
}

mod test {
    #[test]
    fn test_write_store_char_operation() {
        use super::{CharData, CHAR_DATA_SIZE, STORE_CHAR_BUFFER_SIZE};
        use crate::registers::{CharacterMemoryMode, Registers};

        let data: CharData = [0x55u8; CHAR_DATA_SIZE];
        let mut output = [0u8; STORE_CHAR_BUFFER_SIZE];
        assert_eq!(super::build_store_char_operation(&data, 10, &mut output), true);
        assert_eq!(output[0], Registers::CharacterMemoryAddressHigh as u8);
        assert_eq!(output[2], Registers::CharacterMemoryAddressLow as u8);
        assert_eq!(output[2 + CHAR_DATA_SIZE * 4 - 4], Registers::CharacterMemoryAddressLow as u8);
        assert_eq!(output[2 + CHAR_DATA_SIZE * 4], Registers::CharacterMemoryMode as u8);
        assert_eq!(output[2 + CHAR_DATA_SIZE * 4 + 1], CharacterMemoryMode::WriteToNVM as u8);
    }
}
