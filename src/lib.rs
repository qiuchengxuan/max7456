#![no_std]
extern crate peripheral_register;

use peripheral_register::Register;
pub mod registers;
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::spi::{Mode, Phase, Polarity};
use registers::*;

pub const CHAR_DATA_SIZE: usize = 54;
pub const ROW: usize = 16;
pub const COLUMN: usize = 30;

pub type Screen = [[u8; COLUMN as usize]; ROW as usize];

pub const SPI_MODE: Mode = Mode {
    polarity: Polarity::IdleHigh,
    phase: Phase::CaptureOnSecondTransition,
};

pub struct MAX7456<BUS> {
    bus: BUS,
}

pub struct Attributes {
    pub local_background_control: bool,
    pub blink: bool,
    pub revert: bool,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            local_background_control: false,
            blink: false,
            revert: false,
        }
    }
}

impl<'a, E, BUS: Write<u8, Error = E> + Transfer<u8, Error = E>> MAX7456<BUS> {
    pub fn new(bus: BUS) -> Self {
        MAX7456 { bus }
    }

    fn load<T: From<u8>>(&mut self, reg: Registers) -> Result<T, E> {
        let mut value = [0u8; 1];
        self.bus.write(&[reg.read_address()])?;
        self.bus.transfer(&mut value)?;
        Ok(T::from(value[0]))
    }

    pub fn reset<D: DelayMs<u8>>(&mut self, delay: &mut D) -> Result<(), E> {
        let mut video_mode_0: Register<u8, VideoMode0> = Register::of(VideoMode0::SoftwareReset, 1);
        self.bus
            .write(&[Registers::VideoMode0 as u8, video_mode_0.value])?;
        delay.delay_ms(50u8);
        while video_mode_0.get(VideoMode0::SoftwareReset) > 0 {
            video_mode_0 = self.load(Registers::VideoMode0)?;
            delay.delay_ms(1u8);
        }
        Ok(())
    }

    pub fn enable_display(&mut self, enable: bool) -> Result<(), E> {
        let mut video_mode_0: Register<u8, VideoMode0> = self.load(Registers::VideoMode0)?;
        video_mode_0.set(VideoMode0::EnableDisplay, enable as u8);
        self.bus
            .write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn set_standard(&mut self, standard: Standard) -> Result<(), E> {
        let mut video_mode_0: Register<u8, VideoMode0> = self.load(Registers::VideoMode0)?;
        video_mode_0.set(VideoMode0::Standard, standard as u8);
        self.bus
            .write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn clear_display<D: DelayUs<u8>>(&mut self, delay: &mut D) -> Result<bool, E> {
        let mut dmm: Register<u8, DisplayMemoryMode> = self.load(Registers::DisplayMemoryMode)?;
        dmm.set(DisplayMemoryMode::Clear, 1);
        self.bus
            .write(&[Registers::DisplayMemoryMode as u8, dmm.value])?;
        for _ in 0..5 {
            delay.delay_us(20);
            dmm = self.load(Registers::DisplayMemoryMode)?;
            if dmm.get(DisplayMemoryMode::Clear) == 0 {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn display_char<D: DelayMs<u8>>(
        &mut self,
        index: u8,
        data: &[u8; CHAR_DATA_SIZE],
        delay: &mut D,
    ) -> Result<(), E> {
        self.enable_display(false)?;
        let mut operations = [0u8; 2 + CHAR_DATA_SIZE * 4 + 2];
        operations[0] = Registers::CharacterMemoryAddressHigh as u8;
        operations[1] = index;
        for i in 0..data.len() {
            let offset = i * 4;
            operations[offset] = Registers::CharacterMemoryAddressLow as u8;
            operations[offset + 1] = i as u8;
            operations[offset + 2] = Registers::CharacterMemoryDataIn as u8;
            operations[offset + 3] = data[i];
        }
        operations[2 + 54 * 3] = CharacterMemoryMode::WriteToNVM as u8;
        operations[2 + 54 * 3 + 1] = Registers::CharacterMemoryMode as u8;
        self.bus.write(&operations)?;
        delay.delay_ms(12);
        loop {
            let status: Register<u8, Status> = self.load(Registers::Status)?;
            let status = status.get(Status::CharacterMemoryStatus);
            if status == CharacterMemoryStatus::Available as u8 {
                break;
            }
        }
        self.enable_display(true)
    }

    pub fn display_line(
        &mut self,
        row: u8,
        column: u8,
        data: &[u8],
        attributes: Attributes,
    ) -> Result<usize, E> {
        let mut operations = [0u8; 6 + ROW * 2 + 2];
        operations[0] = Registers::DisplayMemoryMode as u8;
        let mut dmm = Register::<u8, DisplayMemoryMode>::new(0);
        dmm.set(
            DisplayMemoryMode::OperationMode,
            OperationMode::Mode16Bit as u8,
        );
        dmm.set(
            DisplayMemoryMode::LocalBackgroundControl,
            attributes.local_background_control as u8,
        );
        dmm.set(DisplayMemoryMode::Blink, attributes.blink as u8);
        dmm.set(DisplayMemoryMode::Invert, attributes.revert as u8);
        dmm.set(DisplayMemoryMode::AutoIncrement, 1);
        operations[1] = dmm.value;

        let address = display_memory_address(row, column);
        operations[2] = Registers::DisplayMemoryAddressHigh as u8;
        operations[3] = (address >> 8) as u8;
        operations[4] = Registers::DisplayMemoryAddressLow as u8;
        operations[5] = address as u8;
        let mut offset = 6;
        let slice = if data.len() <= ROW {
            data
        } else {
            &data[..ROW]
        };
        let mut ff_checker = false;
        for &byte in slice.iter() {
            operations[offset] = Registers::DisplayMemoryDataIn as u8;
            operations[offset + 1] = byte;
            offset += 2;
            ff_checker = byte == 0xFF;
        }
        if ff_checker == true {
            return Ok(0);
        }
        operations[offset] = Registers::DisplayMemoryDataIn as u8;
        operations[offset + 1] = 0xFF;
        self.bus.write(&operations)?;
        Ok(slice.len())
    }

    pub fn display_not_null(&mut self, screen: &Screen, attributes: Attributes) -> Result<(), E> {
        let mut operations = [0u8; 2 + COLUMN * 6];
        operations[0] = Registers::DisplayMemoryMode as u8;
        let mut dmm = Register::<u8, DisplayMemoryMode>::new(0);
        dmm.set(
            DisplayMemoryMode::OperationMode,
            OperationMode::Mode16Bit as u8,
        );
        dmm.set(
            DisplayMemoryMode::LocalBackgroundControl,
            attributes.local_background_control as u8,
        );
        dmm.set(DisplayMemoryMode::Blink, attributes.blink as u8);
        dmm.set(DisplayMemoryMode::Invert, attributes.revert as u8);
        operations[1] = dmm.value;
        let mut offset = 2;
        for row in 0..ROW {
            let line = screen[row];
            for column in 0..COLUMN {
                if line[column] == 0 {
                    continue;
                }
                let address = display_memory_address(row as u8, column as u8);
                operations[offset] = Registers::DisplayMemoryAddressHigh as u8;
                operations[offset + 1] = (address >> 8) as u8;
                operations[offset + 2] = Registers::DisplayMemoryAddressLow as u8;
                operations[offset + 3] = address as u8;
                operations[offset + 4] = Registers::DisplayMemoryDataIn as u8;
                operations[offset + 5] = line[column];
                offset += 6;
                if offset >= operations.len() {
                    self.bus.write(&operations)?;
                    offset = 2;
                }
            }
        }
        if offset > 2 {
            self.bus.write(&operations[..offset])?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod test {
    use embedded_hal::blocking::spi::{Transfer, Write};

    use super::MAX7456;

    struct Bus<'a>(&'a mut [u8]);

    impl<'a> Write<u8> for Bus<'a> {
        type Error = &'static str;
        fn write(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
            if self.0.len() >= bytes.len() {
                let len = bytes.len();
                self.0[..len].copy_from_slice(bytes);
            } else {
                let len = self.0.len();
                self.0.copy_from_slice(&bytes[..len]);
            }
            Ok(())
        }
    }

    impl<'a> Transfer<u8> for Bus<'a> {
        type Error = &'static str;
        fn transfer<'b>(&mut self, bytes: &'b mut [u8]) -> Result<&'b [u8], &'static str> {
            self.write(bytes)?;
            Ok(&[])
        }
    }

    #[test]
    fn test_display_line() {
        let mut buffer = [0u8; 16];
        let bus = Bus(&mut buffer);
        let mut max7456 = MAX7456::new(bus);
        let _result = max7456.display_line(0, 0, b"test", Default::default());
        let actual = format!("{:x?}", buffer);
        let expected = "[4, 1, 5, 0, 6, 0, 7, 74, 7, 65, 7, 73, 7, 74, 7, ff]";
        assert_eq!(expected, actual)
    }
}
