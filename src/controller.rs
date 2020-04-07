use crate::ControllerSettings;
use byteorder::{LittleEndian, ReadBytesExt};
use gilrs::{Axis, Button};
use serde::Deserialize;
use serde_cbor;
use std::error::Error;
use std::io::{self, Read};
use std::net::TcpListener;

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
        let mut car = CarState::new();
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

#[derive(Debug)]
enum TurnDirection {
    Left(f32),
    Right(f32),
    Straight,
}

#[derive(Debug)]
enum DriveDirection {
    Forward(f32),
    Backward(f32),
    Stopped,
}

struct CarState {
    turn: TurnDirection,
    drive: DriveDirection,
}

impl CarState {
    pub fn new() -> Self {
        Self {
            turn: TurnDirection::Straight,
            drive: DriveDirection::Stopped,
        }
    }

    pub fn handle_event(&mut self, event: &GpEvent) {
        match event {
            GpEvent::AxisChanged(Axis::LeftStickX, val) => {
                // Handle turning.
                let val = *val;
                if val < 0.0 {
                    self.turn = TurnDirection::Left(val.abs());
                } else if val > 0.0 {
                    self.turn = TurnDirection::Right(val);
                } else {
                    self.turn = TurnDirection::Straight;
                }
                dbg!(&self.turn);
            }
            GpEvent::ButtonChanged(Button::LeftTrigger2, val) => {
                // Handle reverse.
                self.drive = DriveDirection::Backward(*val);
                dbg!(&self.drive);
            }
            GpEvent::ButtonChanged(Button::RightTrigger2, val) => {
                // Handle forward.
                self.drive = DriveDirection::Forward(*val);
                dbg!(&self.drive);
            }
            _ => (),
        }
    }
}
