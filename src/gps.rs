use crate::GpsSettings;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use nmea0183::{ParseResult, Parser};
use serialport::prelude::*;
use std::error::Error;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::TcpStream;
use std::time::Duration;

struct GpsData {
    latitude: f32,
    longitude: f32,
}

pub struct GpsCollect {
    serial_port: Box<dyn SerialPort>,
    stream: Option<TcpStream>,
}

impl GpsCollect {
    pub fn try_new(gps_settings: GpsSettings) -> Result<Self, Box<dyn Error>> {
        let ip = gps_settings.ip + ":" + &gps_settings.port;
        let serial_settings = SerialPortSettings {
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_secs(1),
        };

        let serial_port = serialport::open_with_settings(
            &gps_settings.device,
            &serial_settings,
        )?;

        let stream = match TcpStream::connect(ip) {
            Ok(conn) => Some(conn),
            Err(e) => {
                eprintln!("Couldn't connect to GPS data socket: {:?}", e);
                None
            }
        };

        Ok(Self {
            stream,
            serial_port,
        })
    }

    pub fn handle_gps(mut self) -> Result<(), Box<dyn Error + Send>> {
        let mut parser = Parser::new();
        let serial_port = BufReader::new(self.serial_port);
        for line in serial_port.lines() {
            match line {
                Ok(mut line) => {
                    line.push('\r');
                    line.push('\n');
                    if let Some(gps_data) =
                        GpsCollect::parse_gps_line(&mut parser, &line)
                    {
                        if let Some(ref mut stream) = self.stream {
                            stream
                                .write_f32::<LittleEndian>(gps_data.latitude)
                                .unwrap();
                            stream
                                .write_f32::<LittleEndian>(gps_data.longitude)
                                .unwrap();
                            stream.flush().unwrap();
                        }
                    }
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
        Ok(())
    }

    fn parse_gps_line(parser: &mut Parser, line: &str) -> Option<GpsData> {
        for result in parser.parse_from_bytes(line.as_bytes()) {
            if let Ok(ParseResult::RMC(Some(msg))) = result {
                let latitude = msg.latitude.as_f64() as f32;
                let longitude = msg.longitude.as_f64() as f32;
                return Some(GpsData {
                    latitude,
                    longitude,
                });
            }
        }
        None
    }
}
