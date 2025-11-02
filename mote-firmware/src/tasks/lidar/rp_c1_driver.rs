use defmt::{Format, error, info, warn};
use embassy_time::{Duration, TimeoutError, Timer, with_timeout};
use embedded_io_async::{ErrorType, ReadExactError};

const START_FLAG: u8 = 0xA5;

#[non_exhaustive]
struct Requests;

impl Requests {
    pub const STOP: [u8; 2] = [START_FLAG, 0x25];
    pub const RESET: [u8; 2] = [START_FLAG, 0x40];
    pub const SCAN: [u8; 2] = [START_FLAG, 0x20];
    pub const EXPRESS_SCAN: [u8; 2] = [START_FLAG, 0x82];
    pub const GET_INFO: [u8; 2] = [START_FLAG, 0x50];
    pub const GET_HEALTH: [u8; 2] = [START_FLAG, 0x52];
    pub const GET_SAMPLE_RATE: [u8; 2] = [START_FLAG, 0x59];
    pub const GET_LIDAR_CONF: [u8; 2] = [START_FLAG, 0x84];
}

#[derive(PartialEq, Eq)]
pub enum LidarState {
    Idle,
    Start,
    Reset,
    CheckHealth,
    ScanRequest,
    ReceiveSample,
    ProcessSample,
    Stop,
}

#[derive(Debug, defmt::Format, Clone, Default, Copy)]
pub struct Point {
    pub quality: u8,
    // Actual heading = angle / 64.0 degrees
    pub angle: u16,
    // Actual distance = distance / 4.0 mm
    pub distance: u16,
}

pub enum ReadSamplesError<T> {
    Timeout,
    CheckBitIncorrect,
    StartFlagIncorrect,
    IoError(T),
}

pub struct RPLidarC1<T>
where
    T: embedded_io_async::Write + embedded_io_async::Read,
{
    connection: T,
}

impl<T> RPLidarC1<T>
where
    T: embedded_io_async::Write + embedded_io_async::Read,
    <T as ErrorType>::Error: Format,
{
    pub fn new(connection: T) -> Self {
        Self { connection }
    }

    async fn clear_read(&mut self) {
        let mut resp = [0; 1];
        let _ = self.connection.flush().await;
        loop {
            if let Err(TimeoutError) = with_timeout(Duration::from_millis(200), self.connection.read(&mut resp)).await {
                break;
            }
        }
    }

    pub async fn reset(&mut self) -> LidarState {
        match self.connection.write_all(&Requests::RESET).await {
            Ok(_) => {
                // Delay to give the LiDAR time to reboot
                Timer::after_millis(1000).await;

                // Clear the UART buffer
                self.clear_read().await;

                return LidarState::CheckHealth;
            }
            Err(err) => {
                // Otherwise we have an error, attempt to reset again after a short delay
                error!("Failed to send RESET command to LiDAR ({}), retrying...", err);
                Timer::after_millis(1000).await;
                return LidarState::Reset;
            }
        }
    }

    pub async fn check_health(&mut self) -> LidarState {
        let mut resp = [0; 10];
        match self.connection.write_all(&Requests::GET_HEALTH).await {
            Ok(()) => match self.connection.read_exact(&mut resp).await {
                Ok(()) => {
                    if resp[0..7] == [0xA5, 0x5A, 0x03, 0x00, 0x00, 0x00, 0x06] {
                        match resp[7] {
                            0x00 => {
                                return LidarState::ScanRequest;
                            }
                            status => {
                                let mut error: [u8; 2] = [0; 2];
                                error.copy_from_slice(&resp[7..10]);
                                error!(
                                    "LiDAR GET_HEALTH returned status code {} and error code {}",
                                    status,
                                    u16::from_le_bytes(error)
                                );
                            }
                        }
                    } else {
                        error!(
                            "LiDAR returned incorrect response to GET_HEALTH message ({:#x}), reseting...",
                            resp
                        );
                    }
                }
                Err(err) => {
                    error!("Failed to read GET_HEALTH response from LiDAR ({})", err);
                }
            },
            Err(err) => {
                error!("Failed to send GET_HEALTH command to LiDAR ({}), reseting...", err);
            }
        }

        return LidarState::Reset;
    }

    pub async fn scan_request(&mut self) -> LidarState {
        let mut resp = [0; 7];
        match self.connection.write_all(&Requests::SCAN).await {
            Ok(()) => match self.connection.read_exact(&mut resp).await {
                Ok(()) => {
                    if resp == [0xA5, 0x5A, 0x05, 0x00, 0x00, 0x40, 0x81] {
                        return LidarState::ReceiveSample;
                    } else {
                        warn!(
                            "LiDAR returned incorrect response to START_SCAN message ({:#x}), checking health...",
                            resp
                        );
                    }
                }
                Err(err) => {
                    warn!(
                        "Failed to read START_SCAN response from LiDAR ({}), checking health...",
                        err
                    );
                }
            },
            Err(err) => {
                warn!(
                    "Failed to send START_SCAN command to LiDAR ({}), checking health...",
                    err
                );
            }
        }
        return LidarState::CheckHealth;
    }

    pub async fn receive_samples<const N: usize>(
        &mut self,
        point_buf: &mut [Point; N],
    ) -> Result<usize, ReadSamplesError<ReadExactError<<T as ErrorType>::Error>>>
    where
        [(); 5 * N]:,
    {
        let mut idx = 0;

        let mut buffer = [0; 5 * N];
        match with_timeout(Duration::from_millis(5000), self.connection.read_exact(&mut buffer)).await {
            Ok(Ok(())) => {
                for i in 0..N {
                    let resp = &buffer[(i * 5)..(i * 5) + 5];
                    if resp[0] & 0b01 == resp[0] & 0b10 {
                        warn!("Start flag data check failed for LiDAR data message.");
                        continue;
                    } else if resp[1] & 0b1 != 1 {
                        error!("Check bit data check failed for LiDAR data message.");
                        continue;
                    } else {
                        let angle = ((resp[2] as u16) << 7) | ((resp[1] as u16 & 0xFE) >> 1);

                        let mut distance_bytes: [u8; 2] = [0; 2];
                        distance_bytes.copy_from_slice(&resp[3..5]);
                        let distance = u16::from_le_bytes(distance_bytes);

                        point_buf[idx] = Point {
                            quality: (resp[0] & !0b11) >> 2,
                            angle: angle,
                            distance: distance,
                        };
                        idx += 1;
                    }
                }

                return Ok(idx);
            }
            Ok(Err(err)) => {
                error!("Failed to read point from LiDAR ({}), reseting...", err);
                return Err(ReadSamplesError::IoError(err));
            }
            Err(TimeoutError) => return Ok(0),
        }
    }
}
