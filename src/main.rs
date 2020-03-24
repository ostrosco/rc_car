use config;
use serde::Deserialize;
use std::error::Error;
use std::thread;

pub mod gps;
use gps::GpsCollect;

pub mod lidar;
use lidar::LidarCollect;

pub mod camera;
use camera::CameraCollect;

#[derive(Clone, Deserialize)]
pub struct CameraSettings {
    ip: String,
    port: String,
}

#[derive(Clone, Deserialize)]
pub struct LidarSettings {
    ip: String,
    port: String,
    device: String,
}

#[derive(Clone, Deserialize)]
pub struct GpsSettings {
    ip: String,
    port: String,
    device: String,
}

#[derive(Deserialize)]
struct Settings {
    camera: CameraSettings,
    lidar: LidarSettings,
    gps: GpsSettings,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("settings")).unwrap();
    let settings_struct = settings.try_into::<Settings>().unwrap();
    let camera_settings = settings_struct.camera.clone();
    let lidar_settings = settings_struct.lidar;
    let gps_settings = settings_struct.gps;

    let camera_thread = thread::spawn(move || {
        let camera_collect = CameraCollect::try_new(camera_settings)
            .expect("Could not start camera connection");
        camera_collect
            .handle_camera()
            .expect("Error handling camera data");
    });

    let lidar_thread = thread::spawn(move || {
        let lidar_collect = LidarCollect::try_new(lidar_settings)
            .expect("Could not start LIDAR connection");
        lidar_collect
            .handle_lidar()
            .expect("Error handling LIDAR data");
    });

    let gps_thread = thread::spawn(move || {
        let gps_collect = GpsCollect::try_new(gps_settings)
            .expect("Could not start GPS connection");
        gps_collect.handle_gps().expect("Error handling GPS data");
    });

    let _ = camera_thread.join();
    let _ = lidar_thread.join();
    let _ = gps_thread.join();
    Ok(())
}
