use i2c_linux::I2c;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

mod fonts;
use crate::fonts::{BasicFont, Font};

pub enum AddressingMode {
    HorizontalMode,
    PageMode,
}

impl Into<u8> for AddressingMode {
    fn into(self) -> u8 {
        match self {
            AddressingMode::HorizontalMode => 0x00,
            AddressingMode::PageMode => 0x02,
        }
    }
}

pub enum OledCommand {
    Address,
    CommandMode,
    DataMode,
    SetAddressingMode,
    DisplayOff,
    DisplayOn,
    NormalDisplay,
    InverseDisplay,
    ActivateScroll,
    DeactivateScroll,
    SetBrightness,
    ScrollLeft,
    ScrollRight,
    Scroll2Frames,
    Scroll3Frames,
    Scroll4Frames,
    Scroll5Frames,
    Scroll25Frames,
    Scroll64Frames,
    Scroll128Frames,
    Scroll256Frames,
}

impl Into<u8> for OledCommand {
    fn into(self) -> u8 {
        match self {
            OledCommand::Address => 0x3d,
            OledCommand::CommandMode => 0x00,
            OledCommand::DataMode => 0x40,
            OledCommand::SetAddressingMode => 0x20,
            OledCommand::DisplayOff => 0xae,
            OledCommand::DisplayOn => 0xaf,
            OledCommand::NormalDisplay => 0xa6,
            OledCommand::InverseDisplay => 0xa7,
            OledCommand::ActivateScroll => 0x2f,
            OledCommand::DeactivateScroll => 0x2e,
            OledCommand::SetBrightness => 0x81,
            OledCommand::ScrollLeft => 0x00,
            OledCommand::ScrollRight => 0x01,
            OledCommand::Scroll2Frames => 0x7,
            OledCommand::Scroll3Frames => 0x4,
            OledCommand::Scroll4Frames => 0x5,
            OledCommand::Scroll5Frames => 0x0,
            OledCommand::Scroll25Frames => 0x6,
            OledCommand::Scroll64Frames => 0x1,
            OledCommand::Scroll128Frames => 0x2,
            OledCommand::Scroll256Frames => 0x3,
        }
    }
}

pub struct Oled {
    device: I2c<File>,
}

impl Oled {
    pub fn from_path<P: AsRef<Path>>(path: P, address: u16) -> Result<Self> {
        let mut i2c = I2c::from_path(path)?;
        i2c.smbus_set_slave_address(address, false)?;
        Ok(Self { device: i2c })
    }

    pub fn setup(&mut self, mode: AddressingMode) -> Result<()> {
        self.send_command(OledCommand::DisplayOff)?;
        self.send_command(0x00)?; // Set lower column address
        self.send_command(0x10)?; // Set higher column address
        self.send_command(0x40)?; // Set display start line
        self.send_command(0xB0)?; // Set page address
        self.send_command(0x81)?; // contrast control
        self.send_command(0xcf)?; // 0~255
        self.send_command(0xa1)?; // Set segment remap
        self.send_command(OledCommand::NormalDisplay)?;
        self.send_command(0xa8)?; // Multiplex ratio
        self.send_command(0x3f)?; // Duty = 1/64
        self.send_command(0xc8)?; // Com scan direction
        self.send_command(0xd3)?; // Set display offset
        self.send_command(0x00)?; //
        self.send_command(0xd5)?; // Set osc division
        self.send_command(0x80)?; //
        self.send_command(0xd9)?; // Set pre-charge period
        self.send_command(0xf1)?; //
        self.send_command(0xda)?; // Set comm pins
        self.send_command(0x12)?; //
        self.send_command(0xdb)?; // Set vcomh
        self.send_command(0x40)?; //
        self.send_command(0x8d)?; // Set charge pump enable
        self.send_command(0x14)?; //
        self.send_command(OledCommand::DisplayOn)?;
        self.set_addressing_mode(mode)?;
        Ok(())
    }

    pub fn send_command<B: Into<u8>>(&mut self, byte: B) -> Result<()> {
        self.device
            .i2c_write_block_data(OledCommand::CommandMode.into(), &[byte.into()])?;
        Ok(())
    }

    pub fn send_data<B: Into<u8>>(&mut self, byte: B) -> Result<()> {
        self.device
            .i2c_write_block_data(OledCommand::DataMode.into(), &[byte.into()])?;
        Ok(())
    }

    pub fn send_array_data<'a, B: Into<&'a [u8]>>(&mut self, data: B) -> Result<()> {
        for chunk in data.into().chunks(31) {
            self.device
                .i2c_write_block_data(OledCommand::DataMode.into(), chunk)?;
        }
        Ok(())
    }

    pub fn set_text_xy(&mut self, column: u8, row: u8) -> Result<()> {
        self.send_command(0xb0 + row)?; // set page address
        self.send_command((8 * column) & 0x0f)?; // set column low address
        self.send_command(0x10 + (((8 * column) >> 4) & 0x0f))?; // set column high address
        Ok(())
    }

    pub fn clear_display(&mut self) -> Result<()> {
        self.send_command(OledCommand::DisplayOff)?;
        for row in 0..8 {
            self.set_text_xy(0, row)?;
            for _ in 0..16 {
                self.put_char(' ')?;
            }
        }
        self.send_command(OledCommand::DisplayOn)?;
        self.set_text_xy(0, 0)?;
        Ok(())
    }

    pub fn put_char(&mut self, char: char) -> Result<()> {
        if let Some(bitmap) = BasicFont::bitmap(char) {
            self.send_array_data(&bitmap[..])?;
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::InvalidData,
                format!("No bitmap for {}", char),
            ))
        }
    }

    pub fn put_string(&mut self, string: &str) -> Result<()> {
        for char in string.chars() {
            self.put_char(char)?;
        }
        Ok(())
    }

    pub fn set_addressing_mode(&mut self, mode: AddressingMode) -> Result<()> {
        self.send_command(OledCommand::SetAddressingMode)?;
        self.send_command(mode)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
