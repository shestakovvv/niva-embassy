use defmt::{trace, warn};
use embassy_futures::select::select;
use embassy_stm32::can::{self, enums::{BusError, FrameCreateError}, CanRx, CanTx, Frame, StandardId};
use embassy_sync::{blocking_mutex::raw::RawMutex, channel::{self}, mutex::Mutex};
use pdo::{RPDO, TPDO};
use rmodbus::server::storage::ModbusStorage;
use sdo::{create_not_implemented_response, create_sdo_abort_response, handle_read_command, handle_unknown_command, handle_write_command, SdoCmd};

mod sdo;
pub mod pdo;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    StorageError(rmodbus::ErrorKind),
    BusError(BusError),
    IncorrectDataLength
}

pub type SharedCanTx<'a, M> = Mutex<M, CanTx<'a>>;


pub struct CanServer<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex + 'static, const CS: usize> {
    node_id: u8,
    can_tx: CanTx<'a>, 
    can_rx: CanRx<'a>, 
    storage: &'a Mutex<M, ModbusStorage<C, D, I, H>>,
    tx_pdo_channel: channel::Receiver<'a, M, TPDO, CS>
}

impl<'a, const C: usize, const D: usize, const I: usize, const H: usize, M: RawMutex, const CS: usize> CanServer<'a, C, D, I, H, M, CS> {
    pub fn new(node_id: u8, can_tx: CanTx<'a>, can_rx: CanRx<'a>, tx_pdo_channel: channel::Receiver<'a, M, TPDO, CS>, storage: &'static Mutex<M, ModbusStorage<C, D, I, H>>) -> Self {
        Self {
            node_id, can_tx, can_rx, storage, tx_pdo_channel
        }
    }

    pub async fn update(&mut self) -> Result<Option<RPDO>, Error> {
        match select(self.can_rx.read(), self.tx_pdo_channel.receive()).await {
            embassy_futures::select::Either::First(res) => {
                let envelope = res.map_err(|e| Error::BusError(e))?;
                match envelope.frame.id() {
                    can::Id::Standard(id) => {
                        if id.as_raw() == 0x600 + self.node_id as u16 {
                            self.process_sdo(self.node_id, envelope.frame.data()).await;
                        } else if id.as_raw() == 0x200 + self.node_id as u16 {
                            return Ok(Some(RPDO::RPDO0(envelope.frame.data().try_into().map_err(|_| Error::IncorrectDataLength)?)));
                        } else if id.as_raw() == 0x300 + self.node_id as u16 {
                            return Ok(Some(RPDO::RPDO1(envelope.frame.data().try_into().map_err(|_| Error::IncorrectDataLength)?)));
                        } else if id.as_raw() == 0x400 + self.node_id as u16 {
                            return Ok(Some(RPDO::RPDO2(envelope.frame.data().try_into().map_err(|_| Error::IncorrectDataLength)?)));
                        } else if id.as_raw() == 0x500 + self.node_id as u16 {
                            return Ok(Some(RPDO::RPDO3(envelope.frame.data().try_into().map_err(|_| Error::IncorrectDataLength)?)));
                        } else {
                            trace!("CanRX: unhandled ({}) {}", id.as_raw(), envelope.frame.data())
                        }
                    },
                    can::Id::Extended(id) => trace!("CanRX: unhandled ({}) {}", id.as_raw(), envelope.frame.data()),
                };
            },
            embassy_futures::select::Either::Second(tpdo) => {
                self.can_tx.write(&tpdo.frame(self.node_id)).await;
            },
        };
        
        Ok(None)
    }

    async fn process_sdo(&mut self, node_id: u8, data: &[u8]) {
        let cmd = SdoCmd::from(data[0]);
        let res = match cmd {
            SdoCmd::Unknown => handle_unknown_command(data, node_id),
            SdoCmd::ReadAny => handle_read_command(cmd, data, node_id, self.storage).await,
            SdoCmd::Read2b => handle_read_command(cmd, data, node_id, self.storage).await,
            SdoCmd::Read4b => handle_read_command(cmd, data, node_id, self.storage).await,
            SdoCmd::Write2b => handle_write_command(cmd, data, node_id, self.storage).await,
            SdoCmd::Write4b => handle_write_command(cmd, data, node_id, self.storage).await,
            _ => create_not_implemented_response(data, node_id).await,
        };
        match res {
            Ok(frame) => {
                self.can_tx.write(&frame).await;
            },
            Err(sdo::Error::SdoAbort(e)) => {
                if let Err(e) = create_sdo_abort_response(data, node_id, e).await {
                    warn!("SdoAbortResponse: {}", e)
                }
            }
            Err(e) => {
                warn!("SdoResponse: {}", e)
            },
        }
    }

    pub fn set_node_id(&mut self, node_id: u8) {
        self.node_id = node_id;
    }

    pub fn node_id(&self) -> u8 {
        self.node_id
    }
}

pub fn create_pdo_frame(node_id: u8, pdo_number: u16, data: &[u8]) -> Result<Frame, FrameCreateError> {
    let pdo_id = (node_id as u16 + (pdo_number / 4)) + 0x180 + (pdo_number%4) * 0x100;
    Frame::new_data(StandardId::new(pdo_id).unwrap(), data)
}