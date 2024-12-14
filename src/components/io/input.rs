mod input_group;
mod digital_input;
mod digital_exti_input;

pub use input_group::DigitalInputGroup;
pub use digital_input::DigitalInput;
pub use digital_exti_input::DigitalExtiInput;


pub enum Polarity {
    Normal,
    Inverse,
}




