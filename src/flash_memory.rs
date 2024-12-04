use embassy_stm32::flash;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Flash(flash::Error),
    NoData,
}

pub mod chunked_sector;