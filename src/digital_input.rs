mod input_group;
mod input;
mod exti_input;

pub use input_group::InputGroup;
pub use input::DigitalInput;
pub use exti_input::DigitalExtiInput;


pub enum Polarity {
    Normal,
    Inverse,
}




