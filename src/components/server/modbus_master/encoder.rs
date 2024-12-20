use defmt::info;
use embassy_futures::select::{self};
use embassy_stm32::usart::{self};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use heapless::Vec;
use rmodbus::{client::ModbusRequest, ModbusProto};

use crate::components::com::rs485::Rs485;

mod regs {
    #![allow(unused)] 

    pub const ROTATION_ANGLE:   u16 = 0;
    pub const ROTATION_ANGLE_F: u16 = 1;
    pub const CURRENT_COUNTER:  u16 = 2;
    pub const ZERO_POINT:       u16 = 3;
    pub const SHAFT_DIAMETER:   u16 = 4;
    pub const LINEAR_SPEED:     u16 = 5;
    pub const ROTATION_FRQ:     u16 = 6;
    pub const NODE_ID:          u16 = 250;
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Timeout,
    UartError(usart::Error),
    ParseError(rmodbus::ErrorKind),
}

const REGS_COUNT: usize = 7;

pub struct Encoder {
    uart: &'static Mutex<ThreadModeRawMutex, Rs485<'static>>,
    node_id: u16,
    timeout: Duration,
}

impl Encoder {
    pub fn new(
        uart: &'static Mutex<ThreadModeRawMutex, Rs485<'static>>,
        node_id: u16,
        timeout: Duration
    ) -> Self {
        Self {
            uart, 
            node_id,
            timeout,
        }
    }

    pub async fn update(&mut self) -> Result<[u16; 5], Error> {
        let mut mreq = ModbusRequest::new(self.node_id as u8, ModbusProto::Rtu);
        let mut request: Vec<u8, 256> = Vec::new();
        let mut response = [0u8; 256];
        mreq.generate_get_holdings(0, REGS_COUNT as u16, &mut request).unwrap();

        let res: select::Either<Result<usize, embassy_stm32::usart::Error>, ()>;
        {
            let mut uart = self.uart.lock().await;
            uart.write(request.as_slice()).await.unwrap();

            res = select::select(
                uart.read_until_idle(&mut response), 
                Timer::after(self.timeout)
            ).await;
        }

        match res {
            select::Either::First(result) => match result {
                Ok(count) => {
                    let mut result: Vec<u16, 7> = Vec::new(); 
                    mreq.parse_u16(&response[..count], &mut result).map_err(|e| Error::ParseError(e))?;
                    Ok([result[0], result[1], result[2], result[5], result[6]])
                },
                Err(e) => {
                    Err(Error::UartError(e))
                },
            },
            select::Either::Second(_) => Err(Error::Timeout),
        }
    }

    async fn set_reg(&self, reg: u16, value: u16) -> Result<(), Error> {
        let mut mreq = ModbusRequest::new(self.node_id as u8, ModbusProto::Rtu);
        let mut request: Vec<u8, 256> = Vec::new();
        let mut response = [0u8; 256];
        mreq.generate_set_holdings_bulk(reg, &[value as u16], &mut request).unwrap();
        info!("value: {} ({})", value, defmt::Debug2Format(&request));

        let res: select::Either<Result<usize, embassy_stm32::usart::Error>, ()>;
        {
            let mut uart = self.uart.lock().await;
            uart.write(&request.as_slice()).await.unwrap();

            res = select::select(
                uart.read_until_idle(&mut response), 
                Timer::after(self.timeout)
            ).await;
        }

        match res {
            select::Either::First(res) => match res {
                Ok(_) => Ok(()),
                Err(e) => Err(Error::UartError(e)),
            },
            select::Either::Second(_) => Err(Error::Timeout),
        }
    }

    async fn reg(&self, reg: u16) -> Result<u16, Error> {
        let mut mreq = ModbusRequest::new(self.node_id as u8, ModbusProto::Rtu);
        let mut request: Vec<u8, 256> = Vec::new();
        let mut response = [0u8; 256];
        mreq.generate_get_holdings(reg, 1, &mut request).unwrap();

        let res: select::Either<Result<usize, embassy_stm32::usart::Error>, ()>;
        {
            let mut uart = self.uart.lock().await;
            uart.write(&request.as_slice()).await.unwrap();

            res = select::select(
                uart.read_until_idle(&mut response), 
                Timer::after(self.timeout)
            ).await;
        }

        match res {
            select::Either::First(result) => match result {
                Ok(count) => {
                    let mut result: Vec<u16, 1> = Vec::new(); 
                    mreq.parse_u16(&response[..count], &mut result).map_err(|e| Error::ParseError(e))?;
                    
                    Ok(result[0])
                },
                Err(e) => {
                    Err(Error::UartError(e))
                },
            },
            select::Either::Second(_) => Err(Error::Timeout),
        }
    }

    pub async fn set_zero_point(&self, zero_point: u16) -> Result<(), Error> {
        self.set_reg(regs::ZERO_POINT, zero_point).await
    }

    pub async fn set_shaft_diameter(&self, shaft_diameter: u16) -> Result<(), Error> {
        self.set_reg(regs::SHAFT_DIAMETER, shaft_diameter).await
    }

    pub async fn set_node_id(&mut self, node_id: u16) -> Result<(), Error> {
        self.set_reg(regs::NODE_ID, node_id).await?;
        self.node_id = node_id;
        Ok(())
    }

    pub async fn zero_point(&self) -> Result<u16, Error> {
        self.reg(regs::ZERO_POINT).await
    }

    pub async fn shaft_diameter(&self) -> Result<u16, Error> {
        self.reg(regs::SHAFT_DIAMETER).await
    }

    pub async fn node_id(&mut self) -> Result<u16, Error> {
        self.reg(regs::NODE_ID).await
    }
}