use crate::ControllerSettings;
use adafruit_motorkit::{DcMotor, MotorControl};
use byteorder::{LittleEndian, ReadBytesExt};
use gilrs::{Axis, Button};
use rppal::gpio::{Gpio, OutputPin};
use serde::Deserialize;
use serde_cbor;
use std::error::Error;
use std::io::{self, Read};
use std::net::TcpListener;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub enum GpEvent {
    ButtonPressed(Button),
    ButtonRepeated(Button),
    ButtonReleased(Button),
    ButtonChanged(Button, f32),
    AxisChanged(Axis, f32),
    Connected,
    Disconnected,
    Dropped,
}
pub struct ControllerCollect {
    listener: TcpListener,
}

impl ControllerCollect {
    pub fn try_new(config: ControllerSettings) -> Result<Self, Box<dyn Error>> {
        let address = format!("0.0.0.0:{}", config.port);
        let listener = TcpListener::bind(address)?;
        Ok(Self { listener })
    }

    pub fn receive_controller(&self) -> io::Result<()> {
        let mut car = CarState::try_new().expect("Couldn't initialize car");
        for stream in self.listener.incoming() {
            match stream {
                Ok(mut stream) => loop {
                    let len = stream.read_u32::<LittleEndian>()? as usize;
                    let mut buffer = vec![0; len];
                    stream
                        .read_exact(&mut buffer[0..len])
                        .expect("Error reading");
                    let device_state: GpEvent =
                        serde_cbor::from_slice(&buffer[0..len])
                            .expect("Error serializing");
                    car.handle_event(&device_state);
                },
                Err(e) => {
                    eprintln!("Error receiving data: {:?}", e);
                }
            }
        }
        Ok(())
    }
}

struct CarState {
    motor_cntl: MotorControl,
    steering: Steering,
    turn: f32,
    drive: f32,
}

impl CarState {
    pub fn try_new() -> Result<Self, Box<dyn Error>> {
        let motor_cntl = MotorControl::try_new(None)?;
        let steering = Steering::try_new(17)?;
        Ok(Self {
            motor_cntl,
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
                self.motor_cntl
                    .set_dc_motor(DcMotor::Motor1, self.drive)
                    .unwrap();
            }
            GpEvent::ButtonChanged(Button::RightTrigger2, val) => {
                // Handle forward.
                self.drive = *val;
                self.motor_cntl
                    .set_dc_motor(DcMotor::Motor1, self.drive)
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
