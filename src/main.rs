use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use config;
use nmea0183::Parser;
use opencv::prelude::Vector;
use opencv::Error as OCVError;
use opencv::{
    core,
    types::{VectorOfint, VectorOfuchar},
    videoio,
};
use rplidar_drv::{RplidarDevice, ScanOptions};
use rpos_drv::Error as RposError;
use serde::Deserialize;
use serialport::prelude::*;
use std::error::Error;
use std::io::BufReader;
use std::io::prelude::*;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

#[derive(Clone, Deserialize)]
struct Camera {
    ip: String,
    port: String,
}

#[derive(Clone, Deserialize)]
struct Lidar {
    ip: String,
    port: String,
    device: String,
}

#[derive(Clone, Deserialize)]
struct Gps {
    ip: String,
    port: String,
    device: String,
}

#[derive(Deserialize)]
struct Settings {
    camera: Camera,
    lidar: Lidar,
    gps: Gps,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("settings")).unwrap();
    let settings_struct = settings.try_into::<Settings>().unwrap();
    let camera_settings = settings_struct.camera.clone();
    let lidar_settings = settings_struct.lidar;
    let gps_settings = settings_struct.gps;

    let camera_thread = thread::spawn(move || -> Result<(), Box<OCVError>> {
        let ip = camera_settings.ip + ":" + &camera_settings.port;
        let mut cam =
            videoio::VideoCapture::new_with_backend(0, videoio::CAP_ANY)?;
        cam.set(
            videoio::CAP_PROP_FOURCC,
            videoio::VideoWriter::fourcc(
                'M' as i8, 'J' as i8, 'P' as i8, 'G' as i8,
            )?
            .into(),
        )?;
        let opened = videoio::VideoCapture::is_opened(&cam)?;
        if !opened {
            panic!("Unable to open default camera!");
        }

        let mut stream = TcpStream::connect(ip)
            .expect("Camera: Cannot connect to sensorview");

        loop {
            let mut frame =
                core::Mat::default().expect("Can't make default frame");
            let mut buf = VectorOfuchar::new();
            cam.read(&mut frame)?;
            if opencv::imgcodecs::imencode(
                ".jpg",
                &frame,
                &mut buf,
                &VectorOfint::new(),
            )? {
                let buf_slice = buf.to_slice();
                let length = buf_slice.len();
                stream
                    .write_all(&length.to_le_bytes())
                    .expect("Couldn't write size");
                stream
                    .write_all(buf_slice)
                    .expect("Camera: Couldn't write buffer");
                stream.flush().expect("Couldn't flush TcpStream");
            } else {
                println!("Couldn't encode image");
            }
        }
    });

    let lidar_thread =
        thread::spawn(move || -> Result<(), Box<dyn Error + Send>> {
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
            )
            .expect("Couldn't open Lidar");
            serial_port
                .write_data_terminal_ready(false)
                .expect("failed to clear DTR");
            let mut rplidar = RplidarDevice::with_stream(serial_port);

            // The default mode of the RPLIDAR A1 will report complete nonsense
            // for the angle. We set it to Standard here to avoid the issue. This
            // likely isn't robust for other LIDARs, however.
            let scan_options = ScanOptions::with_mode(0);

            rplidar
                .start_scan_with_options(&scan_options)
                .expect("Couldn't start scan");

            let mut stream = TcpStream::connect(ip)
                .expect("Lidar: Cannot connect to sensorview");

            loop {
                match rplidar.grab_scan() {
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
                        stream
                            .write_all(&scan_size.to_le_bytes())
                            .expect("Lidar: couldn't write buffer size");
                        stream
                            .write_all(&scan_points_bytes)
                            .expect("Lidar: Couldn't write buffer");
                        stream.flush().expect("Couldn't flush TcpStream");
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
        });

    let gps_thread = thread::spawn(move || -> Result<(), Box<dyn Error + Send>> {
        let serial_settings = SerialPortSettings {
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_secs(1),
        };

        let serial_port = serialport::open_with_settings(&gps_settings.device,
                                                        &serial_settings)
            .expect("Couldn't open serial port for GPS");
        let mut serial_port = BufReader::new(serial_port);
        let mut parser = Parser::new();
        for mut line in serial_port.lines() {
            match line {
                Ok(mut line) => {
                    line.push('\r');
                    line.push('\n');
                    dbg!(&line);
                    for result in parser.parse_from_bytes(line.as_bytes()) {
                        match result {
                            Ok(msg) => println!("{:?}", msg),
                            Err(e) => eprintln!("Error: {:?}", e),
                        }
                    }
                }
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
        Ok(())
    });

    // camera_thread.join().unwrap()?;
    // lidar_thread.join().unwrap().unwrap();
    gps_thread.join().unwrap().unwrap();
    Ok(())
}
