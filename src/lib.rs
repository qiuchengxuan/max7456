#![cfg_attr(not(test), no_std)]

#[cfg(test)]
#[macro_use]
extern crate hex_literal;

extern crate peripheral_register;

pub mod character_memory;
pub mod font;
pub mod incremental_writer;
pub mod lines_writer;
pub mod registers;

use character_memory::{build_store_char_operation, CharData, STORE_CHAR_BUFFER_SIZE};
use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::blocking::spi::{Transfer, Write};
use embedded_hal::spi::{Mode, MODE_3};
use peripheral_register::Register;

use registers::*;

pub const ROW: usize = 16;
pub const COLUMN: usize = 30;

pub const SPI_MODE: Mode = MODE_3;

pub struct MAX7456<SPI> {
    spi: SPI,
}

pub struct Attributes {
    pub local_background_control: bool,
    pub blink: bool,
    pub revert: bool,
}

#[derive(Debug, PartialEq)]
pub struct Display<'a>(pub &'a [u8]);

impl Default for Attributes {
    fn default() -> Self {
        Self { local_background_control: false, blink: false, revert: false }
    }
}

impl<'a, E, SPI: Write<u8, Error = E> + Transfer<u8, Error = E>> MAX7456<SPI> {
    pub fn new(spi: SPI) -> Self {
        MAX7456 { spi }
    }

    pub fn free(self) -> SPI {
        self.spi
    }

    fn load<T: From<u8>>(&mut self, reg: Registers) -> Result<T, E> {
        let mut value = 0u8;
        self.spi.write(core::slice::from_ref(&reg.read_address()))?;
        self.spi.transfer(core::slice::from_mut(&mut value))?;
        Ok(T::from(value))
    }

    pub fn reset(&mut self, delay: &mut dyn DelayMs<u8>) -> Result<(), E> {
        let mut video_mode_0: Register<u8, VideoMode0> = Register::of(VideoMode0::SoftwareReset, 1);
        self.spi.write(&[Registers::VideoMode0 as u8, video_mode_0.value])?;
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
        self.spi.write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn set_standard(&mut self, standard: Standard) -> Result<(), E> {
        let mut video_mode_0: Register<u8, VideoMode0> = self.load(Registers::VideoMode0)?;
        video_mode_0.set(VideoMode0::Standard, standard as u8);
        self.spi.write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn set_sync_mode(&mut self, sync_mode: SyncMode) -> Result<(), E> {
        let mut video_mode_0: Register<u8, VideoMode0> = self.load(Registers::VideoMode0)?;
        video_mode_0.set(VideoMode0::SyncMode, sync_mode as u8);
        self.spi.write(&[Registers::VideoMode0 as u8, video_mode_0.value])
    }

    pub fn set_horizental_offset(&mut self, offset: i8) -> Result<(), E> {
        // -32 ~ +31
        self.spi.write(&[Registers::HorizentalOffset as u8, (offset + 32) as u8])
    }

    pub fn set_vertical_offset(&mut self, offset: i8) -> Result<(), E> {
        // -16 ~ +15
        self.spi.write(&[Registers::VerticalOffset as u8, (offset + 16) as u8])
    }

    pub fn start_clear_display(&mut self) -> Result<(), E> {
        let dmm: Register<u8, DisplayMemoryMode> = Register::of(DisplayMemoryMode::Clear, 1);
        self.spi.write(&[Registers::DisplayMemoryMode as u8, dmm.value])
    }

    pub fn is_display_cleared(&mut self) -> Result<bool, E> {
        let dmm: Register<u8, DisplayMemoryMode> = self.load(Registers::DisplayMemoryMode)?;
        Ok(dmm.get(DisplayMemoryMode::Clear) == 0)
    }

    pub fn wait_clear_display(&mut self, delay: &mut dyn DelayUs<u8>) -> Result<(), E> {
        self.start_clear_display()?;
        delay.delay_us(20);
        while !self.is_display_cleared()? {}
        Ok(())
    }

    pub fn load_char(&mut self, index: u8, output: &mut CharData) -> Result<(), E> {
        self.spi.write(&[
            Registers::CharacterMemoryAddressHigh as u8,
            index,
            Registers::CharacterMemoryMode as u8,
            CharacterMemoryMode::ReadFromNVM as u8,
        ])?;
        for i in 0..64 {
            self.spi.write(&[Registers::CharacterMemoryAddressLow as u8, i as u8])?;
            output[i] = self.load(Registers::CharacterMemoryDataOut)?;
        }
        Ok(())
    }

    pub fn store_char(
        &mut self,
        index: u8,
        data: &CharData,
        delay: &mut dyn DelayMs<u8>,
    ) -> Result<(), E> {
        let mut transaction = [0u8; STORE_CHAR_BUFFER_SIZE];
        build_store_char_operation(data, index, &mut transaction);
        self.spi.write(&transaction)?;
        delay.delay_ms(12);
        loop {
            let status: Register<u8, Status> = self.load(Registers::Status)?;
            let status = status.get(Status::CharacterMemoryStatus);
            if status == CharacterMemoryStatus::Available as u8 {
                break;
            }
        }
        Ok(())
    }

    pub fn write_display(&mut self, display: &Display) -> Result<(), E> {
        self.spi.write(&display.0)
    }
}

#[cfg(test)]
#[macro_use]
extern crate std;
