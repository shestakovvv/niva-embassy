use core::u16;

pub const DEFAULT_RESISTANSE: f32 = 157.7;

pub fn to_millivolts(adc_sample: u16) -> f32 {
    const V_REF: f32 = 3.3; // V
    const V_MAX: f32 = 4.095; // V
    const V_MUL: f32 = V_REF / V_MAX;
    f32::from(adc_sample) / 1000.0 * V_MUL
}

pub fn to_ampers(v: f32, r: f32) -> f32 {
    v / r
}