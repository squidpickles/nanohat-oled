//! An interface for the NanoHat OLED module
#![warn(missing_docs)]
use i2c_linux::I2c;
use std::fs::File;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;

mod font;
use crate::font::BasicFont;

/// The width of the display, in pixels
pub const OLED_WIDTH: u16 = 128;
/// The height of the display, in pixels
pub const OLED_HEIGHT: u16 = 64;
/// The I2C slave address of the display
pub const OLED_ADDRESS: u16 = 0x3c;
/// The height of a single memory page
const OLED_PAGE_HEIGHT: u16 = 8;
/// Prefix for sending a command
const COMMAND_MODE: u8 = 0x00;
/// Prefix for sending bitmap data
const DATA_MODE: u8 = 0x40;
/// Empty array for clearing screen
const EMPTY_SCREEN: [u8; (OLED_WIDTH * OLED_HEIGHT) as usize] =
    [0u8; (OLED_WIDTH * OLED_HEIGHT) as usize];

/// Returned for commands and data sent to the OLED display.
pub type OledResult = Result<()>;

/// For now, images are represented as byte arrays representing 8-bit grayscale bitmaps.
/// Note that images must be exactly the size of the display
pub type Image = [u8];

/// Different addressing modes available for the display.
/// They affect how pointers are advanced after data is written.
pub enum AddressingMode {
    /// Each byte (column) written advances the column pointer by one.
    /// When it reaches the end of the page, the page pointer is advanced
    /// by one, and the column pointer is reset to zero.
    Horizontal,
    /// Each byte (column) written advances the page pointer by one.
    /// When it reaches the last page, the column pointer is advanced
    /// by one, and the page pointer is reset to zero.
    Vertical,
    /// This is the default mode. Each byte (column) written advances the column
    /// pointer by one. When it reaches the end of the page, the column pointer
    /// is reset to zero. The page pointer does not change.
    Page,
}

impl Into<u8> for AddressingMode {
    fn into(self) -> u8 {
        match self {
            AddressingMode::Horizontal => 0x00,
            AddressingMode::Vertical => 0x01,
            AddressingMode::Page => 0x02,
        }
    }
}

/// A command that can be sent to the OLED display
pub enum Command {
    /// Sets contrast level of display, with higher number meaning higher contrast. Default is 0x7f.
    SetContrast,
    /// Display is based on contents of graphics RAM. (default)
    ContentFollowsRam,
    /// Display is all on, regardless of RAM contents.
    EntireDisplayOn,
    /// Sets the addressing mode to one of the [`AddressingMode`](enum.AddressingMode.html)
    /// values.
    SetAddressingMode,
    /// Turns off the display, aka sleep mode. (default)
    DisplayOff,
    /// Turns on the display
    DisplayOn,
    /// Display is white on black, ie a 1 denotes white, 0 denotes black. (default)
    NormalDisplay,
    /// Display is black on white, ie a 1 denotes black, 0 denotes white.
    InverseDisplay,
}

impl Into<u8> for Command {
    fn into(self) -> u8 {
        match self {
            Command::ContentFollowsRam => 0xa4,
            Command::EntireDisplayOn => 0xa5,
            Command::SetAddressingMode => 0x20,
            Command::DisplayOff => 0xae,
            Command::DisplayOn => 0xaf,
            Command::NormalDisplay => 0xa6,
            Command::InverseDisplay => 0xa7,
            Command::SetContrast => 0x81,
        }
    }
}

/// Represents the NanoHat OLED device
pub struct Oled {
    /// Device's I2C slave address
    device: I2c<File>,
}

impl Oled {
    /// Opens the device from its entry in the dev filesystem.
    /// # Example:
    /// ```
    /// # use nanohat_oled::Oled;
    /// let mut oled = Oled::from_path("/dev/i2c-0");
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut i2c = I2c::from_path(path)?;
        i2c.smbus_set_slave_address(OLED_ADDRESS, false)?;
        Ok(Self { device: i2c })
    }

    /// Initial low-level setup for the display
    pub fn init(&mut self) -> OledResult {
        self.send_command(Command::DisplayOff)?;
        self.send_command(0x00)?; // Set lower column address
        self.send_command(0x10)?; // Set higher column address
        self.send_command(0x40)?; // Set display start line
        self.send_command(0xB0)?; // Set page address
        self.send_command(0x81)?; // contrast control
        self.send_command(0x7f)?; // default contrast is 0x7f
        self.send_command(0xa1)?; // Set segment remap
        self.send_command(Command::NormalDisplay)?;
        self.send_command(0xa8)?; // Multiplex ratio
        self.send_command(0x3f)?; // Duty = 1/64
        self.send_command(0xc8)?; // Use remapped COM scan direction
        self.send_command(0xd3)?; // Set display offset
        self.send_command(0x00)?; // No offset
        self.send_command(0xd5)?; // Set display clock division
        self.send_command(0x80)?; // divide ratio
        self.send_command(0xd9)?; // Set pre-charge period
        self.send_command(0xf1)?;
        self.send_command(0xda)?; // Set COM pins
        self.send_command(0x12)?;
        self.send_command(0xdb)?; // Set vcomh deselect level
        self.send_command(0x40)?;
        self.send_command(0x8d)?; // Set charge pump state
        self.send_command(0x14)?; // charge pump enabled
        self.send_command(Command::DisplayOn)?;
        self.set_addressing_mode(AddressingMode::Horizontal)?;
        self.clear_display()?;
        Ok(())
    }

    /// Sends a command or command argument to the display's command parser
    pub fn send_command<B: Into<u8>>(&mut self, byte: B) -> OledResult {
        self.device
            .i2c_write_block_data(COMMAND_MODE, &[byte.into()])?;
        Ok(())
    }

    /// Sends a data byte to the display RAM.
    ///
    /// Display RAM is divided into 8-row pages. When writing bytes to display RAM,
    /// values are set at the current page and column pointer location.
    /// Pixels are set vertically, meaning that for a single byte,
    /// the LSB will be written to the top row of the current page, and the MSB will
    /// be written to the bottom row. Once the byte is written, pointers will advance,
    /// depending on the [`AddressingMode`](enum.AddressingMode.html).
    pub fn send_data<B: Into<u8>>(&mut self, byte: B) -> OledResult {
        self.device
            .i2c_write_block_data(DATA_MODE, &[byte.into()])?;
        Ok(())
    }

    /// Sends a set of data all at once into the display RAM.
    /// Data is always written in chunks of 31 bytes (plus a byte to set data mode).
    /// See [`send_data()`](struct.Oled.html#method.send_data) for more details on RAM layout
    pub fn send_array_data<'a, B: Into<&'a [u8]>>(&mut self, data: B) -> OledResult {
        for chunk in data.into().chunks(31) {
            self.device.i2c_write_block_data(DATA_MODE, chunk)?;
        }
        Ok(())
    }

    /// Sets the cursor position for writing text to display RAM.
    pub fn set_text_xy(&mut self, column: u8, row: u8) -> OledResult {
        self.send_command(0xb0 + row)?; // set page address
        self.send_command((8 * column) & 0x0f)?; // set column low address
        self.send_command(0x10 + (((8 * column) >> 4) & 0x0f))?; // set column high address
        Ok(())
    }

    /// Completely clears the display of text and images
    pub fn clear_display(&mut self) -> OledResult {
        self.send_command(Command::DisplayOff)?;
        self.set_text_xy(0, 0)?;
        self.send_array_data(&EMPTY_SCREEN[..])?;
        self.send_command(Command::DisplayOn)?;
        Ok(())
    }

    /// Writes an image bitmap to the screen.
    /// The bitmap must be the same dimensions as the display.
    /// Anything greater than or equal to the `threshold` will
    /// be interpreted as a `1` pixel; anything under will be
    /// interpreted as a `0`.
    pub fn draw_image(&mut self, image: &Image, threshold: u8) -> OledResult {
        if image.len() != (OLED_HEIGHT * OLED_WIDTH) as usize {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Image dimensions must be {}x{}", OLED_WIDTH, OLED_HEIGHT),
            ));
        }
        let mut write_page = [0u8; (OLED_WIDTH * OLED_HEIGHT / OLED_PAGE_HEIGHT) as usize];
        for (page, page_data) in image
            .chunks((OLED_WIDTH * OLED_PAGE_HEIGHT) as usize)
            .enumerate()
        {
            for (row, row_data) in page_data.chunks(OLED_WIDTH as usize).enumerate() {
                for (column, pixel) in row_data.iter().enumerate() {
                    let pixel = if *pixel >= threshold { 1 } else { 0 };
                    println!(
                        "page: {}, row: {}, column: {}, write offset: {}",
                        page,
                        row,
                        column,
                        (page * OLED_WIDTH as usize) + column
                    );
                    write_page[(page * OLED_WIDTH as usize) + column] |= pixel << row;
                }
            }
        }
        self.set_text_xy(0, 0)?;
        self.send_array_data(&write_page[..])?;
        Ok(())
    }

    /// Writes a single character to the display at the current
    /// X,Y location (as set by [`set_text_xy()`](struct.Oled.html#method.set_text_xy)
    /// and incremented by the [`AddressingMode`](enum.AddressingMode.html)).
    /// Note: only printable ASCII is supported. Other characters will output as
    /// an empty square.
    pub fn put_char(&mut self, char: char) -> OledResult {
        let bitmap = BasicFont::bitmap(char);
        self.send_array_data(&bitmap[..])?;
        Ok(())
    }

    /// Writes a string to the display, starting at the current
    /// X, Y location (as set by `set_text_xy` and incremented by
    /// the [`AddressingMode`](enum.AddressingMode.html)).
    /// None: only printable ASCII is supported
    pub fn put_string(&mut self, string: &str) -> OledResult {
        for char in string.chars() {
            self.put_char(char)?;
        }
        Ok(())
    }

    /// Sets the addressing mode to the supplied [`AddressingMode`](enum.AddressingMode.html).
    /// See [`AddressingMode`](enum.AddressingMode.html) for more details.
    /// Default is [`AddressingMode::Horizontal`](enum.AddressingMode.html#variant.Horizontal).
    pub fn set_addressing_mode(&mut self, mode: AddressingMode) -> OledResult {
        self.send_command(Command::SetAddressingMode)?;
        self.send_command(mode)?;
        Ok(())
    }
}
