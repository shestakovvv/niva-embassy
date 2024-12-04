use defmt::warn;
use embassy_stm32::can::{enums::FrameCreateError, Frame, StandardId};
use embassy_sync::{blocking_mutex::raw::RawMutex, mutex::Mutex};
use heapless::Vec;
use rmodbus::server::{context::ModbusContext, storage::ModbusStorage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SdoCmd {
    Unknown,
    ReadAny,
    Read1B,
    Read2b,
    Read4b,
    Write1B,
    Write2b,
    Write4b,
}

impl From<u8> for SdoCmd {
    fn from(value: u8) -> Self {
        match value {
            0x40 => SdoCmd::ReadAny,
            0x4F => SdoCmd::Read1B,
            0x4B => SdoCmd::Read2b,
            0x43 => SdoCmd::Read4b,
            0x2F => SdoCmd::Write1B,
            0x2B => SdoCmd::Write2b,
            0x23 => SdoCmd::Write4b,
            _ => SdoCmd::Unknown
        }
    }
}

impl Into<u8> for SdoCmd {
    fn into(self) -> u8 {
        match self {
            SdoCmd::ReadAny => 0x40,
            SdoCmd::Read1B => 0x4F,
            SdoCmd::Read2b => 0x4B,
            SdoCmd::Read4b => 0x43,
            SdoCmd::Write1B => 0x2F,
            SdoCmd::Write2b => 0x2B,
            SdoCmd::Write4b => 0x23,
            SdoCmd::Unknown => 0xff
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SdoResponse {
    Unknown,
    Read1B,
    Read2B,
    Read4B,
    WriteSuccess,
    Error
}

impl From<u8> for SdoResponse {
    fn from(value: u8) -> Self {
        match value {
            0x4F => SdoResponse::Read1B,
            0x4B => SdoResponse::Read2B,
            0x43 => SdoResponse::Read4B,
            0x60 => SdoResponse::WriteSuccess,
            0x80 => SdoResponse::Error,
            _ => SdoResponse::Unknown
        }
    }
}

impl Into<u8> for SdoResponse {
    fn into(self) -> u8 {
        match self {
            SdoResponse::Read1B => 0x4F,
            SdoResponse::Read2B => 0x4B,
            SdoResponse::Read4B => 0x43,
            SdoResponse::WriteSuccess => 0x60,
            SdoResponse::Error => 0x80,
            SdoResponse::Unknown => 0xff
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SdoAbortCode {
    InvalidCommand,
    InvalidQuery,
    InvalidSubindex,
    ReadError,
    InvalidData,
    NotImplemented,
    Unknown(u32), // Fallback for unknown codes
}

impl From<u32> for SdoAbortCode {
    fn from(value: u32) -> Self {
        match value {
            0x0000_0001 => SdoAbortCode::InvalidCommand,
            0x0000_0002 => SdoAbortCode::InvalidQuery,
            0x0000_0003 => SdoAbortCode::InvalidSubindex,
            0x0000_0004 => SdoAbortCode::ReadError,
            0x0000_0005 => SdoAbortCode::InvalidData,
            0x0000_00ff => SdoAbortCode::NotImplemented,
            unknown => SdoAbortCode::Unknown(unknown),
        }
    }
}

impl From<SdoAbortCode> for u32 {
    fn from(code: SdoAbortCode) -> Self {
        match code {
            SdoAbortCode::InvalidCommand => 0x0000_0001,
            SdoAbortCode::InvalidQuery => 0x0000_0002,
            SdoAbortCode::InvalidSubindex => 0x0000_0003,
            SdoAbortCode::ReadError => 0x0000_0004, // Keeps unknown codes as they are
            SdoAbortCode::InvalidData => 0x0000_0005, // Keeps unknown codes as they are
            SdoAbortCode::NotImplemented => 0x0000_00ff,
            SdoAbortCode::Unknown(unknown_code) => unknown_code,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) enum Error {
    StandardIdCreateFailed,
    FrameCreateFailed(FrameCreateError),
    NotEnoughData,
    SdoAbort(SdoAbortCode),
    VectorError
}

#[allow(unused)] const CMD: usize = 0;
#[allow(unused)] const RESPONSE_CODE: usize = 0;
#[allow(unused)] const INDEX: usize = 1;
#[allow(unused)] const INDEX_END: usize = 2;
#[allow(unused)] const SUB_INDEX: usize = 3;
#[allow(unused)] const DATA: usize = 4;
#[allow(unused)] const DATA_END: usize = 7;


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SubIndex {
    Coil,
    Discrete,
    Holding,
    Input,
    Unknown(u8)
}

impl From<u8> for SubIndex {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Coil,
            1 => Self::Discrete,
            2 => Self::Holding,
            3 => Self::Input, 
            _ => Self::Unknown(value) 
        }
    }
}


fn check_header_data(data: &[u8]) -> Result<(), Error> {
    if data.len() < 4 {
        return Err(Error::NotEnoughData)
    }
    Ok(())
}

#[inline]
fn new_data_frame(node_id: u8, response_data: &[u8]) -> Result<Frame, Error> {
    Frame::new_data(StandardId::new(0x580+node_id as u16).ok_or(Error::StandardIdCreateFailed)?, response_data).map_err(|e| Error::FrameCreateFailed(e))
}

pub(crate) fn handle_unknown_command(data: &[u8], node_id: u8) -> Result<Frame, Error> {
    let mut response_data = [0u8; 8];
    response_data[0] = SdoResponse::Error as u8;
    response_data[4..].copy_from_slice(&u32::from(SdoAbortCode::InvalidCommand).to_be_bytes());

    if let Ok(_) = check_header_data(data) {
        response_data[1..4].copy_from_slice(&data[1..4]);
    }

    new_data_frame(node_id, &response_data)
}

pub(crate) async fn handle_read_command<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex>(cmd: SdoCmd, data: &[u8], node_id: u8, storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>) -> Result<Frame, Error> {
    let mut response_data = Vec::<u8, 8>::new();
    
    check_header_data(data).map_err(|_| Error::SdoAbort(SdoAbortCode::InvalidQuery))?; 
    response_data.extend_from_slice(&data[CMD..DATA]).map_err(|_| Error::VectorError)?;

    match cmd {
        SdoCmd::Read2b => {
            let v = read_u16(data, storage).await.map_err(|e| Error::SdoAbort(e))?;
            response_data[0] = SdoResponse::Read4B as u8;
            response_data.extend_from_slice(&v.to_be_bytes()).map_err(|_| Error::VectorError)?;
        },
        SdoCmd::Read4b => {
            let v = read_u32(data, storage).await.map_err(|e| Error::SdoAbort(e))?;
            response_data[0] = SdoResponse::Read4B as u8;
            response_data.extend_from_slice(&v.to_be_bytes()).map_err(|_| Error::VectorError)?;
        },
        _ => {
            return Err(Error::SdoAbort(SdoAbortCode::InvalidQuery));
        }
    }
    return new_data_frame(node_id, response_data.as_slice());
}

pub(crate) async fn handle_write_command<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex>(cmd: SdoCmd, data: &[u8], node_id: u8, storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>) -> Result<Frame, Error> {
    let mut response_data = Vec::<u8, 8>::new();

    check_header_data(data).map_err(|_| Error::SdoAbort(SdoAbortCode::InvalidQuery))?;
    response_data.extend_from_slice(&data[CMD..DATA]).map_err(|_| Error::VectorError)?;

    match cmd {
        SdoCmd::Write2b => {
            write_u16(data, storage).await.map_err(|e| Error::SdoAbort(e))?;
            response_data[RESPONSE_CODE] = SdoResponse::WriteSuccess as u8;
            response_data.extend_from_slice(&data[DATA..DATA+size_of::<u16>()]).map_err(|_| Error::VectorError)?;
        },
        SdoCmd::Write4b => {
            write_u32(data, storage).await.map_err(|e| Error::SdoAbort(e))?;
            response_data[RESPONSE_CODE] = SdoResponse::WriteSuccess as u8;
            response_data.extend_from_slice(&data[DATA..DATA+size_of::<u32>()]).map_err(|_| Error::VectorError)?;
        },
        _ => {
            return Err(Error::SdoAbort(SdoAbortCode::InvalidQuery));
        }
    };
    return new_data_frame(node_id, &response_data);
}

async fn read_u32<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex>(data: &[u8], storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>) -> Result<u32, SdoAbortCode> {
    match SubIndex::from(data[SUB_INDEX]) {
        SubIndex::Holding => {
            {
                let storage = storage.lock().await;
                let reg = u16::from_be_bytes([data[INDEX], data[INDEX_END]]);
                Ok(storage.get_holdings_as_u32(reg).map_err(|e| {
                    warn!("SdoProcess: read holdings {}", e);
                    SdoAbortCode::ReadError
                })?)
            }
        },
        SubIndex::Input => {
            {
                let storage = storage.lock().await;
                let reg = u16::from_be_bytes([data[INDEX], data[INDEX_END]]);
                Ok(storage.get_inputs_as_u32(reg).map_err(|e| {
                    warn!("SdoProcess: read inputs {}", e);
                    SdoAbortCode::ReadError
                })?)
            }
        },
        SubIndex::Unknown(_) => {
            Err(SdoAbortCode::InvalidSubindex)
        },
        _ => {
            Err(SdoAbortCode::NotImplemented)
        }
    }
}

async fn read_u16<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex>(data: &[u8], storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>) -> Result<u16, SdoAbortCode> {
    match SubIndex::from(data[SUB_INDEX]) {
        SubIndex::Holding => {
            {
                let storage = storage.lock().await;
                let reg = u16::from_be_bytes([data[INDEX], data[INDEX_END]]);
                Ok(storage.get_holding(reg).map_err(|e| {
                    warn!("SdoProcess: read holdings {}", e);
                    SdoAbortCode::ReadError
                })?)
            }
        },
        SubIndex::Input => {
            {
                let storage = storage.lock().await;
                let reg = u16::from_be_bytes([data[INDEX], data[INDEX_END]]);
                Ok(storage.get_input(reg).map_err(|e| {
                    warn!("SdoProcess: read inputs {}", e);
                    SdoAbortCode::ReadError
                })?)
            }
        },
        SubIndex::Unknown(_) => {
            Err(SdoAbortCode::InvalidSubindex)
        },
        _ => {
            Err(SdoAbortCode::NotImplemented)
        }
    }
}

pub(crate) async fn create_not_implemented_response(data: &[u8], node_id: u8) -> Result<Frame, Error> {
    create_sdo_abort_response(data, node_id, SdoAbortCode::NotImplemented).await
}

pub(crate) async fn create_sdo_abort_response(data: &[u8], node_id: u8, abort_code: SdoAbortCode) -> Result<Frame, Error> {
    let mut response_data = [0u8; 8];
    
    if let Ok(_) = check_header_data(data) {
        response_data[INDEX..DATA].copy_from_slice(&data[INDEX..DATA]);
    } else {
        response_data[RESPONSE_CODE] = SdoResponse::Error as u8;
        response_data[DATA..].copy_from_slice(&u32::from(SdoAbortCode::InvalidQuery).to_be_bytes());
        return new_data_frame(node_id, &response_data);
    }

    response_data[RESPONSE_CODE] = SdoResponse::Error as u8;
    response_data[DATA..].copy_from_slice(&u32::from(abort_code).to_be_bytes());
    return new_data_frame(node_id, &response_data);
}



async fn write_u32<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex>(data: &[u8], storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>) -> Result<u32, SdoAbortCode> {
    match SubIndex::from(data[SUB_INDEX]) {
        SubIndex::Holding => {
            let reg = u16::from_be_bytes([data[INDEX], data[INDEX_END]]);
            let write_data: [u8; 4] = data[DATA..=DATA_END].try_into().map_err(|_| SdoAbortCode::InvalidData)?;
            let write_value = u32::from_be_bytes(write_data);

            let mut storage = storage.lock().await;
            storage.set_holdings_from_u8(reg, &write_value.to_be_bytes()).map_err(|e| {
                warn!("SdoProcess: write holdings {}", e);
                SdoAbortCode::ReadError
            })?;
            Ok(write_value)
        },
        _ => {
            Err(SdoAbortCode::InvalidSubindex)
        }
    }
}

async fn write_u16<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex>(data: &[u8], storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>) -> Result<u16, SdoAbortCode> {
    match SubIndex::from(data[SUB_INDEX]) {
        SubIndex::Holding => {
            let reg = u16::from_be_bytes([data[INDEX], data[INDEX_END]]);
            let write_data: [u8; 2] = data[DATA..=DATA+1].try_into().map_err(|_| SdoAbortCode::InvalidData)?;
            let write_value = u16::from_be_bytes(write_data);

            let mut storage = storage.lock().await;
            storage.set_holdings_from_u8(reg, &write_value.to_be_bytes()).map_err(|e| {
                warn!("SdoProcess: write holdings {}", e);
                SdoAbortCode::ReadError
            })?;
            Ok(write_value)
        },
        _ => {
            Err(SdoAbortCode::InvalidSubindex)
        }
    }
}