use crate::controller::GpEvent;
use adafruit_motorkit::{dc::DcMotor, init_pwm, Motor};
use gilrs::Axis;
use hal::I2cdev;
use linux_embedded_hal as hal;
use pwm_pca9685::Pca9685;
use std::error::Error;

pub struct CarState {
    pwm: Pca9685<I2cdev>,
    left_motor: DcMotor,
    right_motor: DcMotor,
    drive: f32,
}

impl CarState {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let mut pwm = init_pwm(None)?;
        let left_motor = DcMotor::try_new(&mut pwm, Motor::Motor1)?;
        let right_motor = DcMotor::try_new(&mut pwm, Motor::Motor4)?;
        Ok(Self {
            pwm,
            left_motor,
            right_motor,
            drive: 0.0,
        })
    }

    pub fn handle_event(&mut self, event: &GpEvent) {
        match event {
            GpEvent::AxisChanged(Axis::LeftStickY, val) => {
                // Handle reverse.
                self.drive = *val;
                self.left_motor
                    .set_throttle(&mut self.pwm, self.drive)
                    .unwrap();
            }
            GpEvent::AxisChanged(Axis::RightStickY, val) => {
                // Handle forward.
                self.drive = -val;
                self.right_motor
                    .set_throttle(&mut self.pwm, self.drive)
                    .unwrap();
            }
            _ => (),
        }
    }
}
