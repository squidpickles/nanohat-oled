use nanohat_oled::{AddressingMode, Oled};
use std::io::Result;

fn run() -> Result<()> {
    let mut oled = Oled::from_path("/dev/i2c-0", 0x3c)?;
    oled.setup(AddressingMode::HorizontalMode)?;
    oled.clear_display()?;
    oled.put_string("Hello, world")?;
    Ok(())
}

fn main() {
    run().unwrap();
}
