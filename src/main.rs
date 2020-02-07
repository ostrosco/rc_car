use config;
use rscam;
use std::collections::HashMap;
use std::io::prelude::*;
use std::net::TcpStream;

fn main() {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("settings")).unwrap();
    let settings_map = settings.try_into::<HashMap<String, String>>().unwrap();
    let ip = settings_map.get("ip").unwrap().to_owned()
        + ":"
        + settings_map.get("port").unwrap();
    let device = settings_map.get("device").unwrap();

    let mut camera = rscam::new(device).expect("Cannot connect to camera");
    camera
        .start(&rscam::Config {
            interval: (1, 30),
            resolution: (1280, 720),
            format: b"MJPG",
            ..Default::default()
        })
        .expect("Cannot start camera");

    let mut stream =
        TcpStream::connect(ip).expect("Cannot connect to sensorview");

    loop {
        let frame = camera
            .capture()
            .expect("Unable to capture frame from camera");
        let length = frame.len() as u32;
        stream
            .write_all(&length.to_le_bytes())
            .expect("Couldn't write size");
        stream.write_all(&frame[..]).expect("Couldn't write buffer");
        stream.flush().expect("Couldn't flush TcpStream");
    }
}
