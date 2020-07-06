use super::registers::{CharacterMemoryMode, Registers};

pub const CHAR_DATA_SIZE: usize = 64;
pub const STORE_CHAR_BUFFER_SIZE: usize = 2 + CHAR_DATA_SIZE * 4 + 2;

pub type CharData = [u8; CHAR_DATA_SIZE];

pub fn write_store_char_operation(data: &CharData, index: u8, output: &mut [u8]) -> bool {
    if output.len() < STORE_CHAR_BUFFER_SIZE {
        return false;
    }
    output[0] = Registers::CharacterMemoryAddressHigh as u8;
    output[1] = index;
    let output = &mut output[2..];
    for i in 0..data.len() {
        let offset = i * 4;
        output[offset] = Registers::CharacterMemoryAddressLow as u8;
        output[offset + 1] = i as u8;
        output[offset + 2] = Registers::CharacterMemoryDataIn as u8;
        output[offset + 3] = data[i];
    }
    output[CHAR_DATA_SIZE * 4] = Registers::CharacterMemoryMode as u8;
    output[CHAR_DATA_SIZE * 4 + 1] = CharacterMemoryMode::WriteToNVM as u8;
    return true;
}

mod test {
    #[test]
    fn test_write_store_char_operation() {
        use super::{CharData, CHAR_DATA_SIZE, STORE_CHAR_BUFFER_SIZE};
        use crate::registers::{CharacterMemoryMode, Registers};

        let data: CharData = [0x55u8; CHAR_DATA_SIZE];
        let mut output = [0u8; STORE_CHAR_BUFFER_SIZE];
        assert_eq!(super::write_store_char_operation(&data, 10, &mut output), true);
        assert_eq!(output[0], Registers::CharacterMemoryAddressHigh as u8);
        assert_eq!(output[2], Registers::CharacterMemoryAddressLow as u8);
        assert_eq!(output[2 + CHAR_DATA_SIZE * 4 - 4], Registers::CharacterMemoryAddressLow as u8);
        assert_eq!(output[2 + CHAR_DATA_SIZE * 4], Registers::CharacterMemoryMode as u8);
        assert_eq!(output[2 + CHAR_DATA_SIZE * 4 + 1], CharacterMemoryMode::WriteToNVM as u8);
    }
}
