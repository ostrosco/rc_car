use crate::LidarSettings;
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use rplidar_drv::{RplidarDevice, ScanOptions};
use rpos_drv::Error as RposError;
use serialport::prelude::*;
use std::error::Error;
use std::io::prelude::*;
use std::net::TcpStream;
use std::time::Duration;

pub struct LidarCollect {
    rplidar: RplidarDevice<dyn SerialPort>,
    stream: Option<TcpStream>,
}

impl LidarCollect {
    pub fn try_new(
        lidar_settings: LidarSettings,
    ) -> Result<Self, Box<dyn Error>> {
        let ip = lidar_settings.ip + ":" + &lidar_settings.port;
        let serial_settings = SerialPortSettings {
            baud_rate: 115_200,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(1),
        };
        let mut serial_port = serialport::open_with_settings(
            &lidar_settings.device,
            &serial_settings,
        )?;
        serial_port.write_data_terminal_ready(false)?;
        let mut rplidar = RplidarDevice::with_stream(serial_port);

        // The default mode of the RPLIDAR A1 will report complete nonsense
        // for the angle. We set it to Standard here to avoid the issue. This
        // likely isn't robust for other LIDARs, however.
        let scan_options = ScanOptions::with_mode(0);

        rplidar.start_scan_with_options(&scan_options)?;

        let stream = match TcpStream::connect(ip) {
            Ok(conn) => Some(conn),
            Err(e) => {
                eprintln!("Couldn't connect to LIDAR data socket: {:?}", e);
                None
            }
        };

        Ok(Self { stream, rplidar })
    }

    pub fn handle_lidar(mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.rplidar.grab_scan() {
                Ok(mut scan) => {
                    scan.retain(|s| s.dist_mm_q2 > 0);
                    let scan_size = scan.len() as u32;
                    let mut scan_points = vec![];
                    for scan_point in &scan {
                        let angle = scan_point.angle();
                        let distance = scan_point.distance() * 1000.0;
                        scan_points.push((angle, distance));
                    }
                    let mut scan_points_bytes: Vec<u8> = vec![];

                    for dist in scan_points.iter() {
                        scan_points_bytes
                            .write_f32::<LittleEndian>(dist.0)
                            .unwrap();
                        scan_points_bytes
                            .write_f32::<LittleEndian>(dist.1)
                            .unwrap();
                    }
                    if let Some(ref mut stream) = self.stream {
                        stream.write_all(&scan_size.to_le_bytes())?;
                        stream.write_all(&scan_points_bytes)?;
                        stream.flush()?;
                    }
                }
                Err(e) => match e {
                    RposError::OperationTimeout => continue,
                    _ => {
                        println!("Error getting scans from LIDAR: {:?}", e);
                        return Err(Box::new(e));
                    }
                },
            }
        }
    }
}
