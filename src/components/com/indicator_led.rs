use embassy_futures::select;
use embassy_stm32::gpio::Output;
use embassy_sync::{blocking_mutex::raw::RawMutex, signal::Signal};
use embassy_time::{Duration, Timer};

pub struct IndicatorLed<'a, 'b, M: RawMutex> {
    led: Output<'a>,
    signal: &'b Signal<M, bool>,
    short_time: Duration,
    long_time: Duration,
    timeout: Duration,
}

impl <'a, 'b, M: RawMutex> IndicatorLed<'a, 'b, M> {
    pub fn new_advanced(led: Output<'a>, signal: &'b Signal<M, bool>, short_time: Duration, long_time: Duration, timeout: Duration) -> Self {
        Self {
            led, signal, short_time, long_time, timeout
        }
    }

    pub fn new(led: Output<'a>, signal: &'b Signal<M, bool>) -> Self {
        Self {
            led, signal, 
            short_time: Duration::from_millis(40),
            long_time: Duration::from_millis(1000),
            timeout: Duration::from_millis(1000)
        }
    }

    pub fn new_with_timeout(led: Output<'a>, signal: &'b Signal<M, bool>, timeout: Duration) -> Self {
        Self {
            led, signal, 
            timeout,
            short_time: Duration::from_millis(40),
            long_time: Duration::from_millis(1000),
        }
    }

    pub fn short_time(&self) -> Duration {
        self.short_time
    }
    pub fn set_short_time(&mut self, duration: Duration) {
        self.short_time = duration
    }

    pub fn long_time(&self) -> Duration {
        self.long_time
    }
    pub fn set_long_time(&mut self, duration: Duration) {
        self.long_time = duration
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    pub fn set_timeout(&mut self, duration: Duration) {
        self.timeout = duration
    }

    pub async fn update(&mut self) {
        match select::select(self.signal.wait(), Timer::after(self.timeout)).await {
            select::Either::First(s) => {
                if s {
                    self.led.set_low();
                    Timer::after(self.short_time).await;
                    self.led.set_high();
                } else {
                    loop {
                        if let Some(s) = self.signal.try_take() {
                            if s {
                                break;
                            }
                        }
                        self.led.set_high();
                        Timer::after(self.short_time).await;
                        self.led.set_low();
                        Timer::after(self.short_time).await;
                        self.led.set_high();
                        Timer::after(self.short_time).await;
                        self.led.set_low();
                        Timer::after(self.long_time).await;
                    }
                    
                }
                self.signal.reset();
            },
            select::Either::Second(_) => {
                self.led.set_low();
            },
        }
    }

    pub async fn run(&mut self) {
        loop {
            self.update().await
        }
    }
}