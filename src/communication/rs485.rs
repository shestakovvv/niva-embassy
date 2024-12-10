use embassy_stm32::{gpio::Output, mode::Async, usart::{Error, Uart}};

pub struct Rs485<'a> {
    uart: Uart<'a, Async>,
    de: Option<Output<'a>>
}

impl<'a> Rs485<'a> {
    pub fn new(uart: Uart<'a, Async>, de: Option<Output<'a>>) -> Self {
        Self {
            uart,
            de
        }
    }

    pub async fn write(&mut self, buffer: &[u8]) -> Result<(), Error> {
        if let Some(de) = self.de.as_mut() {
            de.set_high();
        }
        self.uart.write(buffer).await?;
        if let Some(de) = self.de.as_mut() {
            de.set_low();
        }
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), Error> {
        self.uart.flush().await
    }

    /// Perform an asynchronous read into `buffer`
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<(), Error> {
        if let Some(de) = self.de.as_mut() {
            de.set_low();
        }
        self.uart.read(buffer).await
    }

    /// Perform an an asynchronous read with idle line detection enabled
    pub async fn read_until_idle(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        if let Some(de) = self.de.as_mut() {
            de.set_low();
        }
        self.uart.read_until_idle(buffer).await
    }
}

