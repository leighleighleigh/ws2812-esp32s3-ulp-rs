use std::error::Error;
use esp_metadata_generated::Chip;

fn main() -> Result<(), Box<dyn Error>> {
    // Determine the name of the configured device:
    let chip = Chip::from_cargo_feature()?;

    // Define all necessary configuration symbols for the configured device:
    chip.define_cfgs();

    // Done!
    Ok(())
}
