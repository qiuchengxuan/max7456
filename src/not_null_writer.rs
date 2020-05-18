use peripheral_register::Register;

use crate::registers::{DisplayMemoryMode, OperationMode, Registers};
use crate::{Attributes, Operations, COLUMN, ROW};

const MAX_ADDRESS: u16 = (ROW * COLUMN) as u16;

pub struct Screen(pub [[u8; COLUMN]; ROW]);

impl Default for Screen {
    fn default() -> Self {
        Self([[0u8; COLUMN]; ROW])
    }
}

pub struct NotNullWriter<'a> {
    screen: &'a Screen,
    attributes: Attributes,
    address: u16,
}

impl<'a> NotNullWriter<'a> {
    pub fn new(screen: &'a Screen, attributes: Attributes) -> Self {
        Self {
            screen,
            attributes,
            address: 0,
        }
    }

    fn dump_bytes(&mut self, limit: u16, buffer: &mut [u8]) -> usize {
        let mut offset = 0;
        while self.address < limit {
            let row = self.address as usize / COLUMN;
            let column = self.address as usize % COLUMN;
            let byte = self.screen.0[row][column];
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

    pub fn write<'b>(&mut self, buffer: &'b mut [u8]) -> Option<Operations<'b>> {
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
                    return Some(Operations(&buffer[..offset]));
                }
            }
        }
        buffer[offset] = Registers::DisplayMemoryAddressHigh as u8;
        buffer[offset + 1] = 1;
        let length = self.dump_bytes(MAX_ADDRESS, &mut buffer[offset + 2..]);
        if length > 0 {
            offset += length + 2;
        }
        if offset > 2 {
            Some(Operations(&buffer[..offset]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::NotNullWriter;
    use super::Screen;

    #[test]
    fn test_low_address() {
        let mut output = [0u8; 32];
        let mut screen = Screen::default();
        screen.0[7][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_high_address() {
        let mut output = [0u8; 32];
        let mut screen = Screen::default();
        screen.0[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_within_addreess() {
        let mut output = [0u8; 32];
        let mut screen = Screen::default();
        screen.0[7][29] = 't' as u8;
        screen.0[8][15] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74, 6, ff, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_cross_address() {
        let mut output = [0u8; 32];
        let mut screen = Screen::default();
        screen.0[7][29] = 't' as u8;
        screen.0[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_exactly_one_buffer() {
        let mut output = [0u8; 14];
        let mut screen = Screen::default();
        screen.0[7][29] = 't' as u8;
        screen.0[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);

        assert_eq!(writer.write(&mut output), None)
    }

    #[test]
    fn test_multiple_buffer() {
        let mut output = [0u8; 8];
        let mut screen = Screen::default();
        screen.0[7][29] = 't' as u8;
        screen.0[8][29] = 't' as u8;
        let mut writer = NotNullWriter::new(&screen, Default::default());
        let expected = "[4, 0, 5, 0, 6, ef, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);

        let expected = "[4, 0, 5, 1, 6, d, 7, 74]";
        let actual = format!("{:x?}", writer.write(&mut output).unwrap().0);
        assert_eq!(actual, expected);
    }
}
