use nanohat_oled::{Oled, OledResult};

fn main() -> OledResult {
    let mut oled = Oled::from_path("/dev/i2c-0")?;
    oled.init()?;
    oled.put_string("Hello, world!")?;
    Ok(())
}
