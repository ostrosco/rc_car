use config;
use opencv::prelude::Vector;
use opencv::{
    core,
    types::{VectorOfint, VectorOfuchar},
    videoio,
};
use std::collections::HashMap;
use std::error::Error;
use std::io::prelude::*;
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("settings")).unwrap();
    let settings_map = settings.try_into::<HashMap<String, String>>().unwrap();
    let ip = settings_map.get("ip").unwrap().to_owned()
        + ":"
        + settings_map.get("port").unwrap();
    let mut cam = videoio::VideoCapture::new_with_backend(0, videoio::CAP_ANY)?;
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

    let mut stream =
        TcpStream::connect(ip).expect("Cannot connect to sensorview");

    loop {
        let mut frame = core::Mat::default().expect("Can't make default frame");
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
            stream.write_all(buf_slice).expect("Couldn't write buffer");
            stream.flush().expect("Couldn't flush TcpStream");
        } else {
            println!("Couldn't encode image");
        }
    }
}
