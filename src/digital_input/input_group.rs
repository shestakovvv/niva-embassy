use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, watch};
use ::num::PrimInt;

use super::DigitalInput;

pub struct InputGroup<'a, const INPUTS_SIZE: usize, const RECEIVER_SIZE: usize, T: 'static + PrimInt> {
    inp: [DigitalInput<'static>; INPUTS_SIZE],
    sender: Option<&'a watch::Sender<'static, ThreadModeRawMutex, T, RECEIVER_SIZE>>,
}

impl<'a, const INPUTS_SIZE: usize, const RECEIVER_SIZE: usize, T: 'static + PrimInt> InputGroup<'a, INPUTS_SIZE, RECEIVER_SIZE, T> {
    pub fn new(
        inp: [DigitalInput<'static>; INPUTS_SIZE],
        sender: Option<&'a watch::Sender<'static, ThreadModeRawMutex, T, RECEIVER_SIZE>>,
    ) -> Self {
        Self { sender, inp }
    }

    pub fn update(&self) -> T {
        let group_value = group_calculate(&self.inp);

        if let Some(sender) = self.sender {
            if let Some(value) = sender.try_get() {
                if value != group_value {
                    sender.send(group_value);
                }
            } else {
                sender.send(group_value);
            }
        }
        
        group_value
    }
}

fn group_calculate<'a, const I: usize, T: PrimInt>(inputs: &[DigitalInput<'a>; I]) -> T {
    let mut array: [bool; I] = [false; I];
    for (index, val) in array.iter_mut().enumerate() {
        *val = inputs[index].is_high();
    }
    array.iter()
        .enumerate()
        .fold(T::from(0).unwrap(), |acc, (i, b)| acc | (process_bool::<T>(*b)) << i)
}

// A helper trait to handle `bool` conversion to numeric types
trait BoolToInt<T> {
    fn to_int(self) -> T;
}

impl<T: PrimInt> BoolToInt<T> for bool {
    fn to_int(self) -> T {
        if self {
            T::one() // Use the numeric `1` for `true`
        } else {
            T::zero() // Use the numeric `0` for `false`
        }
    }
}

fn process_bool<T: PrimInt>(value: bool) -> T {
    value.to_int()
}