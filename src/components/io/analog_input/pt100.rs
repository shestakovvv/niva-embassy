#[cfg(feature = "stm32f405rg")]
use embassy_stm32::adc::{self, RingBufferedAdc};
#[cfg(not(feature = "stm32f405rg"))]
use embassy_stm32::adc::{self, Adc};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use niva_components::{calibration::QuadCalibrationData, data_filter::Kalman};

#[cfg(feature = "stm32f405rg")]
type SharedAdc<T> = Mutex<ThreadModeRawMutex, RingBufferedAdc<'static, T>>;

#[cfg(not(feature = "stm32f405rg"))]
type SharedAdc<T> = Mutex<ThreadModeRawMutex, Adc<'static, T>>;
#[cfg(feature = "stm32f405rg")]
pub struct OverrunError;

const V_PWR: f32 = 5.0; // V

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Pt100CalibrationData {
    pub border: f32,
    pub empty_resistance: f32,
    pub under_border_cal: QuadCalibrationData<f32>,
    pub after_border_cal: QuadCalibrationData<f32>,
}

impl Default for Pt100CalibrationData {
    fn default() -> Self {
        Self { 
            border: 80.0, 
            empty_resistance: 224.0,
            under_border_cal: QuadCalibrationData::default(), 
            after_border_cal: QuadCalibrationData::default(),
        }
    }
}

#[cfg(not(feature = "stm32f405rg"))]
pub struct Pt100<'d, T: adc::Instance, C: adc::AdcChannel<T>> {
    adc: &'d SharedAdc<T>,
    channel: &'d mut C,
    resistors: (f32, f32, f32),
    calibration: Pt100CalibrationData,
    voltage: f32, // V
    resistance: f32, // ohms
    temperature: f32, // Celsius
    kalman: Kalman
}

#[cfg(feature = "stm32f405rg")]
pub struct Pt100<'d, T: adc::Instance, const B: usize> {
    adc: &'d SharedAdc<T>,
    resistors: (f32, f32, f32),
    calibration: Pt100CalibrationData,
    voltage: f32, // V
    resistance: f32, // ohms
    temperature: f32, // Celsius
    kalman: Kalman
}

#[cfg(not(feature = "stm32f405rg"))]
impl<'d, T: adc::Instance, C: adc::AdcChannel<T>> Pt100<'d, T, C> {
    pub fn new(
        adc: &'d SharedAdc<T>,
        channel: &'d mut C,
        resistors: (f32, f32, f32)
    ) -> Self {
        Self {
            adc, 
            channel, 
            resistors,
            calibration: Pt100CalibrationData::default(),
            voltage: 0.0, 
            resistance: 0.0, 
            temperature: 0.0,
            kalman: Kalman::new(0.001, 0.015),
        }
    }

    #[allow(unused)]
    pub fn calibration(&self) -> &Pt100CalibrationData {
        &self.calibration
    }
    #[allow(unused)]
    pub fn set_calibration(&mut self, calibrarion: &Pt100CalibrationData) {
        self.calibration = *calibrarion
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
    pub fn resistance(&self) -> f32 {
        self.resistance
    }
    #[allow(unused)]
    pub fn resistance_as_u16(&self) -> u16 {
        ((self.resistance * 10.0) as i16) as u16
    }
    #[allow(unused)]
    pub fn temperature(&self) -> f32 {
        self.temperature
    }
    #[allow(unused)]
    pub fn temperature_as_u16(&self) -> u16 {
        ((self.temperature * 10.0) as i16) as u16
    }

    pub async fn update(&mut self) {
        let adc_sample: u16;
        {
            let mut adc = self.adc.lock().await;
            adc_sample = adc.read(self.channel).await;
        }
        self.voltage = self.kalman.update(to_millivolts(adc_sample));
        let resistance = to_ohms(self.voltage, (self.resistors.0, self.resistors.1, self.resistors.2));
      
        if resistance <= self.calibration.border {
            self.resistance = self.calibration.under_border_cal.quad_gain*resistance*resistance + self.calibration.under_border_cal.gain*resistance + self.calibration.under_border_cal.bias;
        } else {
            self.resistance = self.calibration.after_border_cal.quad_gain*resistance*resistance + self.calibration.after_border_cal.gain*resistance + self.calibration.after_border_cal.bias;
        }
        self.temperature = to_temperature(self.resistance);
    }

    pub fn is_sensor_connected(&self) -> bool {
        self.temperature < self.calibration.empty_resistance
    }
}



#[cfg(feature = "stm32f405rg")]
impl<'d, T: adc::Instance, const B: usize> Pt100<'d, T, B> {
    pub fn new(
        adc: &'d SharedAdc<T>,
        resistors: (f32, f32, f32)
    ) -> Self {
        Self {
            adc, 
            resistors,
            calibration: Pt100CalibrationData::default(),
            voltage: 0.0, 
            resistance: 0.0, 
            temperature: 0.0,
            kalman: Kalman::new(0.001, 0.015),
        }
    }

    #[allow(unused)]
    pub fn calibration(&self) -> &Pt100CalibrationData {
        &self.calibration
    }
    #[allow(unused)]
    pub fn set_calibration(&mut self, calibrarion: &Pt100CalibrationData) {
        self.calibration = *calibrarion
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
    pub fn resistance(&self) -> f32 {
        self.resistance
    }
    #[allow(unused)]
    pub fn resistance_as_u16(&self) -> u16 {
        ((self.resistance * 10.0) as i16) as u16
    }
    #[allow(unused)]
    pub fn temperature(&self) -> f32 {
        self.temperature
    }
    #[allow(unused)]
    pub fn temperature_as_u16(&self) -> u16 {
        ((self.temperature * 10.0) as i16) as u16
    }

    pub async fn update(&mut self, measurements: &mut [u16; B]) -> Result<f32, OverrunError>  {
        {
            let mut adc = self.adc.lock().await;
            _ = adc.read(measurements).await.map_err(|_| OverrunError )?;
            adc.teardown_adc();
        }
        let mut sum = 0u64;
        for value in measurements.iter() {
            sum += *value as u64
        }
        let adc_sample = (sum / measurements.len() as u64) as u16;

        self.voltage = self.kalman.update(to_millivolts(adc_sample));
        // let (gain, bias) = get_gain_and_bias(PtPort::PT1, storage).await;
        let resistance = to_ohms(self.voltage, (self.resistors.0, self.resistors.1, self.resistors.2));
      
        if resistance <= self.calibration.border {
            self.resistance = self.calibration.under_border_cal.quad_gain*resistance*resistance + self.calibration.under_border_cal.gain*resistance + self.calibration.under_border_cal.bias;
        } else {
            self.resistance = self.calibration.after_border_cal.quad_gain*resistance*resistance + self.calibration.after_border_cal.gain*resistance + self.calibration.after_border_cal.bias;
        }
        self.temperature = to_temperature(self.resistance);
        Ok(self.temperature)
    }

    pub fn is_sensor_connected(&self) -> bool {
        self.temperature < self.calibration.empty_resistance
    }
}


pub fn to_millivolts(adc_sample: u16) -> f32 {
    const V_REF: f32 = 3.3; // V
    const V_MAX: f32 = 4.095; // V
    const V_MUL: f32 = V_REF / V_MAX;
    f32::from(adc_sample) / 1000.0 * V_MUL
}

pub fn to_temperature(ohms: f32) -> f32 {
    // https://intep-komplekt.ru/nsh-termopreobrazovatelej/
    const R0: f32 = 100.0;         // Resistance at 0°C for Pt100
    const ALPHA: f32 = 0.00385;    // Temperature coefficient for platinum

    (ohms - R0) / (ALPHA * R0)
}

pub fn to_ohms(v: f32, r: (f32, f32, f32)) -> f32 {   
    r.2 / ((V_PWR * (r.0+r.1)) / (v * (r.0 as f32)) - 1.0)
}