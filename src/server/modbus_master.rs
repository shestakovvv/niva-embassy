pub mod encoder;

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ModbusSlaves(pub u16, pub u16);

impl From<u32> for ModbusSlaves {
    fn from(value: u32) -> Self {
        Self(value as u16, (value >> 16) as u16)
    }
}

impl From<ModbusSlaves> for u32 {
    fn from(value: ModbusSlaves) -> Self {
        value.0 as u32 | (value.1 as u32) << 16
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SlaveNumber {
    Slave0,
    Slave1,
    Slave2,
    Slave3,
    Slave4,
    Slave5,
    Slave6,
    Slave7,
}

impl From<u8> for SlaveNumber {
    fn from(value: u8) -> Self {
        match value {
            0 => SlaveNumber::Slave0,
            1 => SlaveNumber::Slave1,
            2 => SlaveNumber::Slave2,
            3 => SlaveNumber::Slave3,
            4 => SlaveNumber::Slave4,
            5 => SlaveNumber::Slave5,
            6 => SlaveNumber::Slave6,
            7 => SlaveNumber::Slave7,
            _ => SlaveNumber::Slave0
        }
    }
}

impl From<SlaveNumber> for u8 {
    fn from(value: SlaveNumber) -> Self {
        match value {
            SlaveNumber::Slave0 => 0,
            SlaveNumber::Slave1 => 1,
            SlaveNumber::Slave2 => 2,
            SlaveNumber::Slave3 => 3,
            SlaveNumber::Slave4 => 4,
            SlaveNumber::Slave5 => 5,
            SlaveNumber::Slave6 => 6,
            SlaveNumber::Slave7 => 7,
        }
    }
}