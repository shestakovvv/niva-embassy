use embassy_stm32::gpio::Level;
use embassy_stm32::exti::ExtiInput as EmbExtiInput;

use super::Polarity;

pub struct ExtiInput<'d> {
    exti: EmbExtiInput<'d>,
    polarity: Polarity
}

impl<'d> ExtiInput<'d> {
    #[inline]
    pub fn new(
        input: EmbExtiInput<'d>,
        polarity: Polarity
    ) -> Self {
        Self {
            exti: input,
            polarity
        }
    }

    #[inline]
    pub fn is_high(&self) -> bool {
        match self.polarity {
            Polarity::Inverse => self.exti.is_low(),
            Polarity::Normal => self.exti.is_high(),
        }
    }

    #[inline]
    pub fn is_low(&self) -> bool {
        match self.polarity {
            Polarity::Inverse => self.exti.is_high(),
            Polarity::Normal => self.exti.is_low(),
        }
    }

    #[inline]
    pub fn get_level(&self) -> Level {
        let level = self.exti.get_level();
        match self.polarity {
            Polarity::Inverse => match level {
                embassy_stm32::gpio::Level::Low => embassy_stm32::gpio::Level::High,
                embassy_stm32::gpio::Level::High => embassy_stm32::gpio::Level::Low,
            },
            Polarity::Normal => level,
        }
    }


    /// Asynchronously wait until the pin is high.
    ///
    /// This returns immediately if the pin is already high.
    pub async fn wait_for_high(&mut self) {
        match self.polarity {
            Polarity::Normal => self.exti.wait_for_high().await,
            Polarity::Inverse => self.exti.wait_for_low().await,
        }
    }

    /// Asynchronously wait until the pin is low.
    ///
    /// This returns immediately if the pin is already low.
    pub async fn wait_for_low(&mut self) {
        match self.polarity {
            Polarity::Normal => self.exti.wait_for_low().await,
            Polarity::Inverse => self.exti.wait_for_high().await,
        }
    }

    /// Asynchronously wait until the pin sees a rising edge.
    ///
    /// If the pin is already high, it will wait for it to go low then back high.
    pub async fn wait_for_rising_edge(&mut self) {
        match self.polarity {
            Polarity::Normal => self.exti.wait_for_rising_edge().await,
            Polarity::Inverse => self.exti.wait_for_falling_edge().await,
        }
    }

    /// Asynchronously wait until the pin sees a falling edge.
    ///
    /// If the pin is already low, it will wait for it to go high then back low.
    pub async fn wait_for_falling_edge(&mut self) {
        match self.polarity {
            Polarity::Normal => self.exti.wait_for_falling_edge().await,
            Polarity::Inverse => self.exti.wait_for_rising_edge().await,
        }
    }

    /// Asynchronously wait until the pin sees any edge (either rising or falling).
    pub async fn wait_for_any_edge(&mut self) {
        self.exti.wait_for_any_edge().await
    }
}