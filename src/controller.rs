use crate::{ControllerSettings, car::CarState};
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

