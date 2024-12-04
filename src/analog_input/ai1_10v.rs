use core::u16;

#[cfg(feature = "stm32f405rg")]
use embassy_stm32::adc::{self, RingBufferedAdc};
#[cfg(not(feature = "stm32f405rg"))]
use embassy_stm32::adc::{self, Adc};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use niva_components::data_filter::Kalman;

#[cfg(feature = "stm32f405rg")]
type SharedAdc<T> = Mutex<ThreadModeRawMutex, RingBufferedAdc<'static, T>>;

#[cfg(not(feature = "stm32f405rg"))]
type SharedAdc<T> = Mutex<ThreadModeRawMutex, Adc<'static, T>>;
#[cfg(feature = "stm32f405rg")]
pub struct OverrunError;

pub const MIN_VOLTAGE: f32 = 1.0;
pub const MAX_VOLTAGE: f32 = 10.0;

#[cfg(not(feature = "stm32f405rg"))]
pub struct AI1_10<'d, T: adc::Instance, C: adc::AdcChannel<T>> {
    adc: &'d SharedAdc<T>,
    channel: &'d mut C,
    raw_voltage: f32, // V
    voltage: f32, // V
    kalman: Kalman
}

#[cfg(feature = "stm32f405rg")]
pub struct AI1_10<'d, T: adc::Instance, const S: usize> {
    adc: &'d SharedAdc<T>,
    raw_voltage: f32, // V
    voltage: f32, // V
    kalman: Kalman
}

#[cfg(not(feature = "stm32f405rg"))]
impl<'d, T: adc::Instance, C: adc::AdcChannel<T>> AI1_10<'d, T, C> {
    pub fn new(
        adc: &'d SharedAdc<T>,
        channel: &'d mut C,
    ) -> Self {
        Self {
            adc, channel,
            raw_voltage: 0.0, 
            voltage: 0.0, 
            kalman: Kalman::new(0.001, 0.015),
        }
    }

    #[allow(unused)]
    pub fn voltage(&self) -> f32 {
        self.voltage
    }
    #[allow(unused)]
    pub fn voltage_as_u16(&self) -> u16 {
        ((self.voltage * 1000.0) as i16) as u16
    }

    #[allow(unused)]
    pub fn raw_voltage(&self) -> f32 {
        self.raw_voltage
    }
    #[allow(unused)]
    pub fn raw_voltage_as_u16(&self) -> u16 {
        ((self.voltage * 1000.0) as i16) as u16
    }
    

    pub async fn update(&mut self) {
        let adc_sample: u16;
        {
            let mut adc = self.adc.lock().await;
            adc_sample = adc.read(self.channel).await;
        }
        self.raw_voltage = self.kalman.update(to_millivolts(adc_sample));
        self.voltage = to_voltage(self.raw_voltage);
    }

    pub fn is_sensor_connected(&self) -> bool {
        self.voltage >= MIN_VOLTAGE
    }
}

#[cfg(feature = "stm32f405rg")]
impl<'d, T: adc::Instance, const S: usize> AI1_10<'d, T, S> {
    pub fn new(
        adc: &'d SharedAdc<T>,
    ) -> Self {
        Self {
            adc,
            raw_voltage: 0.0, 
            voltage: 0.0, 
            kalman: Kalman::new(0.001, 0.015),
        }
    }

    #[allow(unused)]
    pub fn voltage(&self) -> f32 {
        self.voltage
    }
    #[allow(unused)]
    pub fn voltage_as_u16(&self) -> u16 {
        ((self.voltage * 1000.0) as i16) as u16
    }

    #[allow(unused)]
    pub fn raw_voltage(&self) -> f32 {
        self.raw_voltage
    }
    #[allow(unused)]
    pub fn raw_voltage_as_u16(&self) -> u16 {
        ((self.voltage * 1000.0) as i16) as u16
    }
    

    pub async fn update(&mut self, measurements: &mut [u16; S]) -> Result<f32, OverrunError> {
        let size: usize;
        {
            let mut adc = self.adc.lock().await;
            size = adc.read(measurements).await.map_err(|_| OverrunError )?;
        }
        let mut sum = 0u64;
        for i in 0..size {
            sum += measurements[i] as u64
        }
        let adc_sample = (sum / size as u64) as u16;

        self.raw_voltage = self.kalman.update(to_millivolts(adc_sample));
        self.voltage = to_voltage(self.raw_voltage);
        Ok(self.voltage)
    }

    pub fn is_sensor_connected(&self) -> bool {
        self.voltage >= MIN_VOLTAGE
    }
}


pub fn to_millivolts(adc_sample: u16) -> f32 {
    const V_REF: f32 = 3.3; // V
    const V_MAX: f32 = 4.095; // V
    const V_MUL: f32 = V_REF / V_MAX;
    f32::from(adc_sample) / 1000.0 * V_MUL
}

pub fn to_voltage(v: f32) -> f32 {
    const K: f32 = 3.333333;     // Resistance at 0°C for Pt100

    v * K
}