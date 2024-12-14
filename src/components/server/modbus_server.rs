use defmt::{trace, Debug2Format};
use embassy_stm32::usart::{self, Uart};
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use heapless::Vec;
use rmodbus::{server::{storage::ModbusStorage, ModbusFrame}, ModbusFrameBuf, ModbusProto};

const MODBUS_BUF_SIZE: usize = 256;

#[derive(Debug)]
pub enum Error {
    ModbusProcess(rmodbus::ErrorKind),
    Uart(usart::Error)
}

pub struct ModbusServer<const C: usize, const D: usize, const I: usize, const H: usize> {
    uart: Uart<'static, embassy_stm32::mode::Async>,
    storage: &'static Mutex<ThreadModeRawMutex, ModbusStorage<C,D,I,H>>,
}

impl<const C: usize, const D: usize, const I: usize, const H: usize> ModbusServer<C,D,I,H> {
    pub fn new(uart: Uart<'static, embassy_stm32::mode::Async>, storage: &'static Mutex<ThreadModeRawMutex, ModbusStorage<C,D,I,H>>) -> Self {
        Self {
            uart,
            storage,
        }
    }

    pub async fn update(&mut self, id: u8) -> Result<(), Error> {
        let mut buf: ModbusFrameBuf = [0; MODBUS_BUF_SIZE];
        let count = self.uart.read_until_idle(&mut buf).await.map_err(|e| Error::Uart(e))?;
        trace!("ModbusServer: RX {}", &buf[..count]);
        let mut response: Vec<u8, MODBUS_BUF_SIZE> = Vec::new();
        let mut frame = ModbusFrame::new(id, &buf[..count], ModbusProto::Rtu, &mut response);
        frame.parse().map_err(|e| Error::ModbusProcess(e))?;
        if frame.processing_required {
            if frame.readonly {
                let storage_locked = self.storage.lock().await;
                frame.process_read(&*storage_locked).map_err(|e| Error::ModbusProcess(e))?;
            } else {
                let mut storage_locked = self.storage.lock().await;
                frame.process_write(&mut *storage_locked).map_err(|e| Error::ModbusProcess(e))?;
            };
        }
        if frame.response_required {
            frame.finalize_response().unwrap();
            trace!("ModbusServer: TX {}", Debug2Format(&response));
            self.uart.write(response.as_slice()).await.map_err(|e| Error::Uart(e))?;
        }
        Ok(())
    }
}