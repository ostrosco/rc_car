use rppal::gpio::{Gpio, OutputPin};
use std::time::Duration;
use adafruit_motorkit::{dc::DcMotor, init_pwm, Motor};
use linux_embedded_hal as hal;
use hal::I2cdev;
use pwm_pca9685::{Pca9685};
use std::error::Error;
use crate::controller::GpEvent;
use gilrs::{Axis, Button};

pub struct CarState {
    pwm: Pca9685<I2cdev>,
    dc_motor: DcMotor,
    steering: Steering,
    turn: f32,
    drive: f32,
}

impl CarState {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let mut pwm = init_pwm(None)?;
        let dc_motor = DcMotor::try_new(&mut pwm, Motor::Motor1)?;
        let steering = Steering::try_new(17)?;
        Ok(Self {
            pwm,
            dc_motor,
            steering,
            turn: 0.0,
            drive: 0.0,
        })
    }

    pub fn handle_event(&mut self, event: &GpEvent) {
        match event {
            GpEvent::AxisChanged(Axis::LeftStickX, val) => {
                // Handle turning.
                self.turn = *val;
                self.steering.steer(self.turn).unwrap();
            }
            GpEvent::ButtonChanged(Button::LeftTrigger2, val) => {
                // Handle reverse.
                self.drive = -val;
                self.dc_motor
                    .set_throttle(&mut self.pwm, self.drive)
                    .unwrap();
            }
            GpEvent::ButtonChanged(Button::RightTrigger2, val) => {
                // Handle forward.
                self.drive = *val;
                self.dc_motor
                    .set_throttle(&mut self.pwm, self.drive)
                    .unwrap();
            }
            _ => (),
        }
    }
}

// These values were calculated experimentally for the Spectrum SPMS401.
const MIN_PERIOD: f32 = 1300.0;
const MAX_PERIOD: f32 = 1700.0;

struct Steering {
    pin: OutputPin,
}

impl Steering {
    pub fn try_new(pin_num: u8) -> Result<Self, Box<dyn Error>> {
        let gpio = Gpio::new()?;
        let pin = gpio.get(pin_num)?.into_output();
        Ok(Self { pin })
    }

    pub fn steer(&mut self, val: f32) -> Result<(), Box<dyn Error>> {
        // Normalize the value from [-1.0, 1.0] to [0.0, 1.0]. This also
        // assumes that the axis reading from the joystick follows typical
        // X/Y coordinates (hence left is negative and right is positive).
        // In testing, I found that the value was flipped on the servo,
        // hence the negation of the received value here before normalization.
        let val = (-val + 1.0) / 2.0;

        // Normalize the values from the joystick to [MIN_PERIOD, MAX_PERIOD].
        let period = (MIN_PERIOD + (MAX_PERIOD - MIN_PERIOD) * val) as u64;
        self.pin.set_pwm(
            Duration::from_millis(20),
            Duration::from_micros(period),
        )?;
        Ok(())
    }
}
