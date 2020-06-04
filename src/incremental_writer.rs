use peripheral_register::Register;

use crate::registers::{DisplayMemoryMode, OperationMode, Registers};
use crate::{display_memory_address, Attributes, Display};

pub struct IncrementalWriter<'a> {
    bytes: &'a [u8],
    address: u16,
    attributes: Attributes,
    index: usize,
}

impl<'a> IncrementalWriter<'a> {
    pub fn new(bytes: &'a [u8], row: u8, column: u8, attributes: Attributes) -> Self {
        Self { bytes, address: display_memory_address(row, column), attributes, index: 0 }
    }

    pub fn remain(&self) -> usize {
        self.bytes.len() - self.index
    }

    pub fn write<'b>(&mut self, buffer: &'b mut [u8]) -> Option<Display<'b>> {
        assert!(buffer.len() >= 10);

        buffer[0] = Registers::DisplayMemoryMode as u8;
        let mut dmm = Register::<u8, DisplayMemoryMode>::new(0);
        dmm.set(DisplayMemoryMode::OperationMode, OperationMode::Mode16Bit as u8);
        dmm.set(
            DisplayMemoryMode::LocalBackgroundControl,
            self.attributes.local_background_control as u8,
        );
        dmm.set(DisplayMemoryMode::Blink, self.attributes.blink as u8);
        dmm.set(DisplayMemoryMode::Invert, self.attributes.revert as u8);
        dmm.set(DisplayMemoryMode::AutoIncrement, 1);
        buffer[1] = dmm.value;

        buffer[2] = Registers::DisplayMemoryAddressHigh as u8;
        buffer[3] = (self.address >> 8) as u8;
        buffer[4] = Registers::DisplayMemoryAddressLow as u8;
        buffer[5] = self.address as u8;

        let mut offset = 6;
        let mut written = 0;
        let mut ff_checker = false;
        for &byte in self.bytes[self.index..].iter() {
            buffer[offset] = Registers::DisplayMemoryDataIn as u8;
            buffer[offset + 1] = byte;
            ff_checker = byte == 0xFF;
            written += 1;
            offset += 2;
            if offset + 2 >= buffer.len() {
                break;
            }
        }
        if ff_checker {
            return None;
        }
        buffer[offset] = Registers::DisplayMemoryDataIn as u8;
        buffer[offset + 1] = 0xFF;
        self.index += written;
        self.address += written as u16;
        Some(Display(&buffer[..offset + 2]))
    }
}

#[cfg(test)]
mod test {
    use super::IncrementalWriter;

    #[test]
    fn test_functional() {
        let mut output = [0u8; 32];
        let mut writer = IncrementalWriter::new(b"test", 0, 0, Default::default());
        let expected = "[4, 1, 5, 0, 6, 0, 7, 74, 7, 65, 7, 73, 7, 74, 7, ff]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_breaks() {
        let mut output = [0u8; 6 + 26 + 2];
        let upper_letters = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut writer = IncrementalWriter::new(upper_letters, 0, 0, Default::default());
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        let expected = "[4, 1, 5, 0, 6, 0, 7, 41, 7, 42, 7, 43, 7, 44, 7, 45, 7, 46, 7, 47, 7, \
                         48, 7, 49, 7, 4a, 7, 4b, 7, 4c, 7, 4d, 7, ff]";
        assert_eq!(actual, expected);
        assert_eq!(writer.remain() > 0, true);
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        let expected = "[4, 1, 5, 0, 6, d, 7, 4e, 7, 4f, 7, 50, 7, 51, 7, 52, 7, 53, 7, 54, 7, \
                         55, 7, 56, 7, 57, 7, 58, 7, 59, 7, 5a, 7, ff]";
        assert_eq!(actual, expected);
        assert_eq!(writer.remain(), 0);
    }
}
