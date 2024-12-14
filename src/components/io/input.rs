mod input_group;
mod input;
mod exti_input;

pub use input_group::InputGroup;
pub use input::Input;
pub use exti_input::ExtiInput;


pub enum Polarity {
    Normal,
    Inverse,
}




