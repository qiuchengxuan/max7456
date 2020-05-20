use core::cmp::min;

use peripheral_register::Register;

use crate::registers::{DisplayMemoryMode, OperationMode, Registers};
use crate::{Attributes, Display, COLUMN, ROW};

const MAX_ADDRESS: u16 = (ROW * COLUMN) as u16;

pub struct NotNullWriter<'a, T> {
    screen: &'a [T],
    attributes: Attributes,
    address: u16,
}

impl<'a, T: AsRef<[u8]>> NotNullWriter<'a, T> {
    pub fn new(screen: &'a [T], attributes: Attributes) -> Self {
        Self {
            screen,
            attributes,
            address: 0,
        }
    }

    fn dump_bytes(&mut self, limit: u16, buffer: &mut [u8]) -> usize {
        let mut offset = 0;
        let max_column = min(self.screen[0].as_ref().len(), COLUMN);
        let real_limit = min(
            limit,
            ((self.screen.len() - 1) * COLUMN + max_column) as u16,
        );
        while self.address < real_limit {
            let row = self.address as usize / COLUMN;
            let column = self.address as usize % COLUMN;
            if column >= max_column {
                self.address += 1;
                continue;
            }
            let byte = self.screen[row].as_ref()[column];
            if byte == 0 {
                self.address += 1;
                continue;
            }
            buffer[offset] = Registers::DisplayMemoryAddressLow as u8;
            buffer[offset + 1] = self.address as u8;
            buffer[offset + 2] = Registers::DisplayMemoryDataIn as u8;
            buffer[offset + 3] = byte;
            offset += 4;
            self.address += 1;
            if offset + 4 >= buffer.len() {
                break;
            }
        }
        return offset;
    }

    pub fn write<'b>(&mut self, buffer: &'b mut [u8]) -> Display<'b> {
        assert!(buffer.len() >= 8);

        buffer[0] = Registers::DisplayMemoryMode as u8;
        let mut dmm = Register::<u8, DisplayMemoryMode>::new(0);
        dmm.set(
            DisplayMemoryMode::OperationMode,
            OperationMode::Mode16Bit as u8,
        );
        dmm.set(
            DisplayMemoryMode::LocalBackgroundControl,
            self.attributes.local_background_control as u8,
        );
        dmm.set(DisplayMemoryMode::Blink, self.attributes.blink as u8);
        dmm.set(DisplayMemoryMode::Invert, self.attributes.revert as u8);
        buffer[1] = dmm.value;
        let mut offset = 2;
        if self.address < 0x100 {
            buffer[2] = Registers::DisplayMemoryAddressHigh as u8;
            buffer[3] = 0;
            let length = self.dump_bytes(0x100, &mut buffer[4..]);
            if length > 0 {
                offset += length + 2;
                if offset + 6 > buffer.len() {
                    return Display(&buffer[..offset]);
                }
            }
        }
        buffer[offset] = Registers::DisplayMemoryAddressHigh as u8;
        buffer[offset + 1] = 1;
        let length = self.dump_bytes(MAX_ADDRESS, &mut buffer[offset + 2..]);
        if length > 0 {
            offset += length + 2;
        }
        if offset + 1 < buffer.len() {
            buffer[offset + 1] = 0; // in case of revert
        }

        Display(&buffer[..if offset > 2 { offset } else { 0 }])
    }
}

pub fn revert(buffer: &mut [u8]) -> Display {
    if buffer[0] != Registers::DisplayMemoryMode as u8 {
        return Display(&buffer[..0]);
    }
    for i in 1..buffer.len() / 2 {
        match buffer[i * 2] {
            b if b == Registers::DisplayMemoryDataIn as u8 => buffer[i * 2 + 1] = 0,
            b if b == Registers::CharacterMemoryAddressLow as u8 => continue,
            b if b == Registers::CharacterMemoryAddressHigh as u8 => continue,
            0 => return Display(&buffer[..(i - 1) * 2]),
            _ => return Display(&buffer[..0]),
        }
    }
    Display(buffer)
}

#[cfg(test)]
mod test {
    use super::NotNullWriter;

    #[test]
    fn test_low_address() {
        let mut output = [0u8; 32];
        let mut screen = [[0u8; 30]; 16];
        screen[7][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_high_address() {
        let mut output = [0u8; 32];
        let mut screen = [[0u8; 30]; 16];
        screen[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_within_addreess() {
        let mut output = [0u8; 32];
        let mut screen = [[0u8; 30]; 16];
        screen[7][29] = 't' as u8;
        screen[8][15] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74, 6, ff, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cross_address() {
        let mut output = [0u8; 32];
        let mut screen = [[0u8; 30]; 16];
        screen[7][29] = 't' as u8;
        screen[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_exactly_one_buffer() {
        let mut output = [0u8; 14];
        let mut screen = [[0u8; 30]; 16];
        screen[7][29] = 't' as u8;
        screen[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);

        assert_eq!(writer.write(&mut output).0.len(), 0)
    }

    #[test]
    fn test_multiple_buffer() {
        let mut output = [0u8; 8];
        let mut screen = [[0u8; 30]; 16];
        screen[7][29] = 't' as u8;
        screen[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);

        let expected = "[4, 0, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_non_standard_screen() {
        let mut output = [0u8; 8];
        let mut screen = [[0u8; 29]; 15];
        screen[7][28] = 't' as u8;
        screen[8][28] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ee, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);

        let expected = "[4, 0, 5, 1, 6, c, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).0);
        assert_eq!(actual, expected);
    }
}
