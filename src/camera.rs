use crate::CameraSettings;
use opencv::prelude::Vector;
use opencv::{
    core,
    types::{VectorOfint, VectorOfuchar},
    videoio::{self, VideoCapture},
};
use std::error::Error;
use std::io::Write;
use std::net::TcpStream;

pub struct CameraCollect {
    cam: VideoCapture,
    stream: Option<TcpStream>,
}

impl CameraCollect {
    pub fn try_new(
        camera_settings: CameraSettings,
    ) -> Result<Self, Box<dyn Error>> {
        let ip = camera_settings.ip + ":" + &camera_settings.port;
        let mut cam = VideoCapture::new_with_backend(0, videoio::CAP_ANY)?;
        cam.set(
            videoio::CAP_PROP_FOURCC,
            videoio::VideoWriter::fourcc(
                'M' as i8, 'J' as i8, 'P' as i8, 'G' as i8,
            )?
            .into(),
        )?;
        let opened = VideoCapture::is_opened(&cam)?;
        if !opened {
            panic!("Unable to open default camera!");
        }

        let stream = match TcpStream::connect(ip) {
            Ok(conn) => Some(conn),
            Err(e) => {
                eprintln!("Couldn't connect to camera data socket: {:?}", e);
                None
            }
        };

        Ok(Self { cam, stream })
    }

    pub fn handle_camera(mut self) -> Result<(), Box<dyn Error>> {
        loop {
            let mut frame = core::Mat::default()?;
            let mut buf = VectorOfuchar::new();
            self.cam.read(&mut frame)?;
            if opencv::imgcodecs::imencode(
                ".jpg",
                &frame,
                &mut buf,
                &VectorOfint::new(),
            )? {
                let buf_slice = buf.to_slice();
                let length = buf_slice.len();
                if let Some(ref mut stream) = self.stream {
                    stream.write_all(&length.to_le_bytes())?;
                    stream.write_all(buf_slice)?;
                    stream.flush()?;
                }
            } else {
                println!("Couldn't encode image");
            }
        }
    }
}
