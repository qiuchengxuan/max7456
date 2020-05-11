#![no_std]
extern crate peripheral_register;
use core::cell::Cell;

use peripheral_register::Register;
pub mod registers;
use registers::*;

pub const CHAR_DATA_SIZE: usize = 54;
pub const ROW: usize = 30;
pub const COLUMN: usize = 16;

pub type Screen = [[u8; COLUMN as usize]; ROW as usize];

pub trait Spi {
    type Error;
    fn write(&self, bytes: &[u8]) -> Result<(), Self::Error>;
    fn write_read(&self, input: &[u8], output: &mut [u8]) -> Result<(), Self::Error>;
}

pub trait DelayUs {
    fn delay_us(&self, us: usize);
}

pub struct MAX7456<'a, S, D> {
    bus: Cell<&'a S>,
    delay: Cell<&'a D>,
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

impl<'a, S: Spi, D: DelayUs> MAX7456<'a, S, D> {
    pub fn new(bus: &'a S, delay: &'a D) -> Self {
        MAX7456 {
            bus: Cell::new(bus),
            delay: Cell::new(delay),
        }
    }

    fn load<T: From<u8>>(&self, reg: Registers) -> Result<T, S::Error> {
        let bus = self.bus.get();
        let mut value = [0u8; 1];
        bus.write_read(&[reg.read_address()], &mut value)?;
        Ok(T::from(value[0]))
    }

    pub fn disable_display(&self, disable: bool) -> Result<(), S::Error> {
        let mut video_mode_0: Register<u8, VideoMode0> = self.load(Registers::VideoMode0)?;
        video_mode_0.set(VideoMode0::EnableDisplay, disable as u8);
        let bus = self.bus.get();
        bus.write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn set_standard(&self, standard: Standard) -> Result<(), S::Error> {
        let mut video_mode_0: Register<u8, VideoMode0> = self.load(Registers::VideoMode0)?;
        video_mode_0.set(VideoMode0::Standard, standard as u8);
        let bus = self.bus.get();
        bus.write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn clear_display(&self) -> Result<(), S::Error> {
        let mut dmm: Register<u8, DisplayMemoryMode> = self.load(Registers::DisplayMemoryMode)?;
        dmm.set(DisplayMemoryMode::Clear, 1);
        let bus = self.bus.get();
        bus.write(&[Registers::DisplayMemoryMode as u8, dmm.value])?;
        let delay = self.delay.get();
        delay.delay_us(20);
        while dmm.get(DisplayMemoryMode::Clear) > 0 {
            dmm = self.load(Registers::DisplayMemoryMode)?;
        }
        Ok(())
    }

    pub fn write_char(&self, index: u8, data: &[u8; CHAR_DATA_SIZE]) -> Result<(), S::Error> {
        self.disable_display(true)?;
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
        let bus = self.bus.get();
        bus.write(&operations)?;
        let delay = self.delay.get();
        delay.delay_us(12 * 1000);
        loop {
            let status = Register::<u8, Status>::new(0);
            bus.write_read(&[Registers::Status.read_address()], status.into())?;
            let status = status.get(Status::CharacterMemoryStatus);
            if status == CharacterMemoryStatus::Available as u8 {
                break;
            }
        }
        self.disable_display(false)
    }

    pub fn display_line(
        &self,
        row: u8,
        column: u8,
        data: &[u8],
        attributes: Attributes,
    ) -> Result<usize, S::Error> {
        let mut operations = [0u8; 6 + ROW * 2 + 1];
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
        let slice = if data.len() > ROW { data } else { &data[..ROW] };
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
        operations[offset] = 0xFF;
        let bus = self.bus.get();
        bus.write(&operations)?;
        Ok(slice.len())
    }

    pub fn display_not_null(
        &self,
        screen: &Screen,
        attributes: Attributes,
    ) -> Result<(), S::Error> {
        let bus = self.bus.get();
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
                    bus.write(&operations)?;
                    offset = 2;
                }
            }
        }
        if offset > 2 {
            bus.write(&operations[..offset])?;
        }
        Ok(())
    }
}
