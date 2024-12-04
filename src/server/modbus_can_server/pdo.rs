use embassy_stm32::can::{enums::FrameCreateError, frame::ClassicData, Frame, StandardId};

#[derive(Debug, Clone, Copy)]
pub enum RPDO {
    RPDO0([u8; 8]),
    RPDO1([u8; 8]),
    RPDO2([u8; 8]),
    RPDO3([u8; 8]),
}

#[derive(Debug, Clone, Copy)]
pub struct TPDO {
    number: u8,
    size: usize,
    data: ClassicData
}

impl TPDO {
    pub fn new(number: u8, data: &[u8]) -> Result<TPDO, FrameCreateError> {
        Ok(Self {
            number, size: data.len(), data: ClassicData::new(data)?
        })
    }

    pub fn frame(&self, node_id: u8) -> Frame {
        let number = self.number as u16;
        let cob_id = (node_id as u16 + (number / 4)) + 0x180 + (number%4) * 0x100;
        Frame::new_data(StandardId::new(cob_id).unwrap(), &self.data.raw()[..self.size]).unwrap()
    }
}
