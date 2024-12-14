pub mod ai1_10v;
pub mod ai4_20ma;
pub mod pt100;

#[inline]
#[allow(unused)]
pub fn volt_to_u16(v: f32) -> u16 {
    (v * 1000.0) as u16
}

#[inline]
#[allow(unused)]
pub fn milliampere_to_u16(v: f32) -> u16 {
    (v * 1_000_000.0) as u16
}

#[inline]
#[allow(unused)]
pub fn celsius_to_u16(v: f32) -> u16 {
    (v * 10.0) as u16
}