use embassy_stm32::gpio::{Input, Level};

use super::Polarity;

pub struct DigitalInput<'d> {
    input: Input<'d>,
    polarity: Polarity
}

impl<'d> DigitalInput<'d> {
    #[inline]
    pub fn new(input: Input<'d>, polarity: Polarity) -> Self {
        Self {
            input,
            polarity
        }
    }

    #[inline]
    pub fn is_high(&self) -> bool {
        match self.polarity {
            Polarity::Inverse => self.input.is_low(),
            Polarity::Normal => self.input.is_high(),
        }
    }

    #[inline]
    pub fn is_low(&self) -> bool {
        match self.polarity {
            Polarity::Inverse => self.input.is_high(),
            Polarity::Normal => self.input.is_low(),
        }
    }

    #[inline]
    pub fn get_level(&self) -> Level {
        let level = self.input.get_level();
        match self.polarity {
            Polarity::Inverse => match level {
                embassy_stm32::gpio::Level::Low => embassy_stm32::gpio::Level::High,
                embassy_stm32::gpio::Level::High => embassy_stm32::gpio::Level::Low,
            },
            Polarity::Normal => level,
        }
    }
}