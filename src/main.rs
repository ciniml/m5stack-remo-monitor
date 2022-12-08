
#[cfg(target_os="espidf")]
pub mod comm_esp;
#[cfg(target_os="espidf")]
mod espidf_imports {
    pub use super::comm_esp::*;
    pub use embedded_svc::{wifi::{ClientConnectionStatus, ClientIpStatus, ClientStatus, Wifi}, timer::{TimerService, PeriodicTimer}, http::client::Response};
    pub use esp_idf_hal::prelude::Peripherals;
    pub use esp_idf_svc::{netif::EspNetifStack, wifi::EspWifi, timer::{EspTimerService, EspTaskTimerService, EspTimer}};
    pub use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
    pub use embedded_svc::http::Headers;
}
#[cfg(target_os="espidf")]
use espidf_imports::*;

#[cfg(target_os="linux")]
mod comm_linux;
#[cfg(target_os="linux")]
use comm_linux::*;

use std::{sync::{Arc, Mutex}, time::Duration, str::FromStr, fmt::Write};

use anyhow::anyhow;
use embedded_io::blocking::Read;

use rand::prelude::*;

use lgfx::{self, ColorRgb332};
use heapless::Vec;

const LOGO_PNG: &[u8; 9278] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/rust-logo-512x512-blk_white.png"
));

use lgfx::{DrawImage, DrawPrimitives, Gfx};
use uuid::Uuid;

use crate::lgfx::{EpdMode, DrawChars, FontManupulation, LgfxDisplay};

const MAX_DEVICES: usize = 32;
const MAX_APPLIANCES: usize = 16;

use remo_api::{Device, read_devices, DeviceSubNode, NewestEvents, EchonetLiteProperty, Appliance, read_appliances, ApplianceSubNode};

mod config;
use config::*;

#[derive(Clone, Copy, Debug)]
struct SensorRecord
{
    pub ambient_temperature: f32,
    pub relative_humidity: f32,
    pub ambient_luminous_level: f32,
    pub instant_power_usage: f32,
}

impl Default for SensorRecord {
    fn default() -> Self {
        Self {
            ambient_temperature: 0.0,
            relative_humidity: 0.0,
            ambient_luminous_level: 0.0,
            instant_power_usage: 0.0,
        }
    }
}

//type Timestamp = std::time::SystemTime;
type Timestamp = chrono::DateTime<chrono::Utc>;
fn timestamp_now() -> Timestamp { chrono::Utc::now() }

struct SensorRecords<const N: usize>
{
    records: heapless::spsc::Queue<SensorRecord, N>,
    timestamp: Option<Timestamp>,
}

impl<const N: usize> SensorRecords<N>
{
    pub const fn new() -> Self {
        Self {
            records: heapless::spsc::Queue::new(),
            timestamp: None,
        }
    }

    pub fn add_with_timestamp(&mut self, record: SensorRecord, timestamp: Timestamp) {
        if self.records.is_full() {
            self.records.dequeue();
        }
        self.records.enqueue(record).unwrap();
        self.timestamp = Some(timestamp);
    }

    pub fn last_timestamp(&self) -> Option<Timestamp> {
        self.timestamp
    }

    pub fn iter<'a>(&'a self) -> heapless::spsc::Iter<'a, SensorRecord, N> {
        self.records.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn latest(&self) -> Option<(SensorRecord, Timestamp)> {
        if let Some(timestamp) = self.timestamp {
            self.records.iter().last().and_then(|record| Some((*record, timestamp)))
        } else {
            None
        }
    }
}

const SENSOR_RECORD_CAPACITY: usize = 60*24+1;
static mut SENSOR_RECORDS: SensorRecords<SENSOR_RECORD_CAPACITY> = SensorRecords::new();
static LAST_RECORD: std::sync::Mutex<Option<(SensorRecord, Timestamp)>> = std::sync::Mutex::new(None);
static SAMPLED_RECORD: std::sync::Mutex<Option<(SensorRecord, Timestamp)>> = std::sync::Mutex::new(None);
static LAST_RATE_LIMIT: std::sync::Mutex<RateLimitInfo> = std::sync::Mutex::new(RateLimitInfo::new());

fn update_task_random(_wifi: Arc<Mutex<EspWifi>>) -> ! {
    let mut rng = rand::thread_rng();
    loop {
        let record = SensorRecord {
            ambient_temperature: rng.gen_range(0.0..40.0),
            relative_humidity: rng.gen_range(0.0..=100.0),
            ambient_luminous_level: rng.gen_range(0.0..=100.0),
            instant_power_usage: rng.gen_range(0.0..2000.0),
        };
        let timestamp = timestamp_now();
        log::info!("update task: {:?} {:?}", record, timestamp);
        *LAST_RECORD.lock().unwrap() = Some((record, timestamp));
        std::thread::sleep(Duration::from_secs(30));
    }
}

fn fetch_remo_sensor_data() -> anyhow::Result<(SensorRecord, Timestamp, RateLimitInfo)> {
    let mut record = SensorRecord::default();
    let mut timestamp = timestamp_now();
    let ((_, newest_events), _) = get_target_device()?;
    
    if let Some(events) = newest_events {
        if let Some(temperature) = events.temperature {
            record.ambient_temperature = temperature.val;
            timestamp = temperature.created_at;
        }
        if let Some(humidity) = events.humidity {
            record.relative_humidity = humidity.val;
            timestamp = humidity.created_at;
        }
        if let Some(luminous) = events.illumination {
            record.ambient_luminous_level = luminous.val;
            timestamp = luminous.created_at;
        }
    }

    let ((_, properties), rate_limit) = get_target_appliance()?;
    let instant: Option<u32> = properties.iter().find(|property| property.epc == 231 )
        .and_then(|property| property.val.parse().ok());
    let coefficient: Option<u32> = properties.iter().find(|property| property.epc == 211 )
        .and_then(|property| property.val.parse().ok());
    if let Some(instant_power) = instant.and_then(|instant| coefficient.and_then(|coefficient| Some(instant*coefficient))) {
        record.instant_power_usage = instant_power as f32;
    }
    Ok((record, timestamp, rate_limit))
}

fn update_task_cloudapi(wifi: Arc<Mutex<EspWifi>>) -> ! {
    let mut prev_has_network_connection = false;
    loop {
        let has_network_connection = {
            log::info!("Waiting WiFi lock...");
            let wifi = wifi.lock().unwrap();
            log::info!("Waiting WiFi gets non-transitional state...");
            let result = wifi.wait_status_with_timeout(Duration::from_secs(10), |status| !status.is_transitional());
            log::info!("WiFi gots non-transitional state - {:?}", result);

            match wifi.get_status().0 {
                ClientStatus::Started(connection_status) => {
                    match connection_status {
                        ClientConnectionStatus::Connected(ip_status) => {
                            match ip_status {
                                ClientIpStatus::Done(settings) => {
                                    if !prev_has_network_connection {
                                        log::info!("Got IP address: {:?}", settings)
                                    }
                                    true
                                },
                                _ => false,
                            }
                        },
                        ClientConnectionStatus::Disconnected => {
                            if prev_has_network_connection {
                                log::info!("Disconnected from AP");
                            }
                            false
                        },
                        _ => false,
                    }
                },
                _ => false,
            }
        };
        prev_has_network_connection = has_network_connection;

        if !has_network_connection {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        }

        match fetch_remo_sensor_data() {
            Ok((record, timestamp, rate_limit)) => {
                *LAST_RECORD.lock().unwrap() = Some((record, timestamp));
                *LAST_RATE_LIMIT.lock().unwrap() = rate_limit;
            },
            Err(err) => {
                log::error!("fetch sensor data failed: {:?}", err);
            }
        }
        std::thread::sleep(Duration::from_secs(30));
    }
}

fn sample_task() {
    let timestamp =  timestamp_now();
    log::info!("sample task: {:?}", timestamp);
    let last_record = LAST_RECORD.lock().unwrap().take();
    *SAMPLED_RECORD.lock().unwrap() = last_record.and_then(|record| Some((record.0, timestamp)));
}

fn ui_task(gfx: lgfx::SharedLgfxTarget) -> ! {
    let sensor_records = unsafe { &mut SENSOR_RECORDS };
    loop {
        if let Some((record, timestamp)) = SAMPLED_RECORD.lock().unwrap().take() {
            // New record has arrived.
            sensor_records.add_with_timestamp(record, timestamp);
        }
        let rate_limit = LAST_RATE_LIMIT.lock().unwrap().clone();

        let (max, min) = if sensor_records.is_empty() {
            // Default max/min
            let max = SensorRecord {
                ambient_temperature: 40.0,
                relative_humidity: 100.0,
                ambient_luminous_level: 100.0,
                instant_power_usage: 1000.0,
            };
            let min = SensorRecord {
                ambient_temperature: 0.0,
                relative_humidity: 0.0,
                ambient_luminous_level: 0.0,
                instant_power_usage: 0.0,
            };
            (max, min)
        } else {
            // Calculate max/min
            let mut max = SensorRecord {
                ambient_temperature: f32::NEG_INFINITY,
                relative_humidity: f32::NEG_INFINITY,
                ambient_luminous_level: f32::NEG_INFINITY,
                instant_power_usage: f32::NEG_INFINITY,
            };
            let mut min = SensorRecord {
                ambient_temperature: f32::INFINITY,
                relative_humidity: f32::INFINITY,
                ambient_luminous_level: f32::INFINITY,
                instant_power_usage: f32::INFINITY,
            };
            for record in sensor_records.iter() {
                max.ambient_temperature = max.ambient_temperature.max(record.ambient_temperature);
                max.relative_humidity = max.relative_humidity.max(record.relative_humidity);
                max.ambient_luminous_level = max.ambient_luminous_level.max(record.ambient_luminous_level);
                max.instant_power_usage = max.instant_power_usage.max(record.instant_power_usage);
                min.ambient_temperature = min.ambient_temperature.min(record.ambient_temperature);
                min.relative_humidity = min.relative_humidity.min(record.relative_humidity);
                min.ambient_luminous_level = min.ambient_luminous_level.min(record.ambient_luminous_level);
                min.instant_power_usage = min.instant_power_usage.min(record.instant_power_usage);
            }
            (max, min)
        };

        let mut min_temperature_str = heapless::String::<16>::new();
        let mut cur_temperature_str = heapless::String::<16>::new();
        let mut max_temperature_str = heapless::String::<16>::new();
        let mut min_humidity_str = heapless::String::<16>::new();
        let mut cur_humidity_str = heapless::String::<16>::new();
        let mut max_humidity_str = heapless::String::<16>::new();
        let mut min_power_str = heapless::String::<16>::new();
        let mut cur_power_str = heapless::String::<16>::new();
        let mut max_power_str = heapless::String::<16>::new();
        let mut timestamp_str = heapless::String::<32>::new();
        let mut rate_limit_str = heapless::String::<64>::new();

        let latest = sensor_records.latest();
        if let Some((record, timestamp)) = latest {
            use std::fmt::Write;
            write!(&mut min_temperature_str, "{:6.1}", min.ambient_temperature).ok();
            write!(&mut cur_temperature_str, "{:6.1}", record.ambient_temperature).ok();
            write!(&mut max_temperature_str, "{:6.1}", max.ambient_temperature).ok();
            write!(&mut min_humidity_str, "{:6.1}", min.relative_humidity).ok();
            write!(&mut cur_humidity_str, "{:6.1}", record.relative_humidity).ok();
            write!(&mut max_humidity_str, "{:6.1}", max.relative_humidity).ok();
            write!(&mut min_power_str, "{:6.1}", min.instant_power_usage).ok();
            write!(&mut cur_power_str, "{:6.1}", record.instant_power_usage).ok();
            write!(&mut max_power_str, "{:6.1}", max.instant_power_usage).ok();
            write!(&mut timestamp_str, "{:?}", timestamp).ok();
        } else {
            use std::fmt::Write;
            min_temperature_str.write_str("--").ok();
            cur_temperature_str.write_str("--").ok();
            max_temperature_str.write_str("--").ok();
            min_humidity_str.write_str("--").ok();
            cur_humidity_str.write_str("--").ok();
            max_humidity_str.write_str("--").ok();
            min_power_str.write_str("--").ok();
            cur_power_str.write_str("--").ok();
            max_power_str.write_str("--").ok();
            timestamp_str.write_str("--").ok();
        }

        {
            write!(&mut rate_limit_str, "rate limit: ").ok();
            if let Some(limit) = rate_limit.limit {
                if let Some(remaining) = rate_limit.remaining {
                    write!(&mut rate_limit_str, "{}/{}", remaining, limit).ok();
                }
            }
            if let Some(reset) = rate_limit.reset {
                let timestamp = Timestamp::from_utc(chrono::NaiveDateTime::from_timestamp(reset as i64, 0), chrono::Utc);
                write!(&mut rate_limit_str, " reset at: {}", timestamp.format("%Y-%m-%d %H:%M:%S"));
            }
        }
        {
            let mut guard = gfx.lock_without_auto_update();
            let foreground = ColorRgb332::new(0xff);
            let background = ColorRgb332::new(0x00);
            guard.set_font(lgfx::fonts::FreeMono24pt7b).ok();
            let font_height = guard.font_height();
            let line_height = font_height * 9 / 8;
            guard.clear(lgfx::ColorRgb332::new(0xff));
            let mut y_offset = font_height;
            let value_margin_left = 20;
            let value_width = 200;
            guard.draw_chars("Temperature:", 0, y_offset, foreground, background, 0.75, 0.75);
            y_offset += line_height;
            guard.draw_chars(&min_temperature_str, value_margin_left + value_width * 0, y_offset, foreground, background, 1.0, 1.0);
            guard.draw_chars(&cur_temperature_str, value_margin_left + value_width * 1, y_offset, foreground, background, 1.0, 1.0);
            guard.draw_chars(&max_temperature_str, value_margin_left + value_width * 2, y_offset, foreground, background, 1.0, 1.0);
            y_offset += line_height*5/4;
            
            guard.draw_chars("Humidity:", 0, y_offset, foreground, background, 0.75, 0.75);
            y_offset += line_height;
            guard.draw_chars(&min_humidity_str, value_margin_left + value_width * 0, y_offset, foreground, background, 1.0, 1.0);
            guard.draw_chars(&cur_humidity_str, value_margin_left + value_width * 1, y_offset, foreground, background, 1.0, 1.0);
            guard.draw_chars(&max_humidity_str, value_margin_left + value_width * 2, y_offset, foreground, background, 1.0, 1.0);
            y_offset += line_height*5/4;

            guard.draw_chars("Power:", 0, y_offset, foreground, background, 0.75, 0.75);
            y_offset += line_height;
            guard.draw_chars(&min_power_str, value_margin_left + value_width * 0, y_offset, foreground, background, 1.0, 1.0);
            guard.draw_chars(&cur_power_str, value_margin_left + value_width * 1, y_offset, foreground, background, 1.0, 1.0);
            guard.draw_chars(&max_power_str, value_margin_left + value_width * 2, y_offset, foreground, background, 1.0, 1.0);

            guard.draw_chars(&rate_limit_str, 0, 540 - line_height, foreground, background, 0.75, 0.75);
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}

static GFX: std::sync::Mutex<Option<Gfx>> = std::sync::Mutex::new(None);
static SAMPLE_TIMER_SERVICE: std::sync::Mutex<Option<EspTaskTimerService>> = std::sync::Mutex::new(None);
static SAMPLE_TIMER: std::sync::Mutex<Option<EspTimer>> = std::sync::Mutex::new(None);
fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    #[cfg(target_os="espidf")]
    {
        esp_idf_sys::link_patches();
        esp_idf_svc::log::EspLogger::initialize_default();
    }

    println!("Hello, world!");
    #[cfg(target_os="espidf")]
    {
        *GFX.lock().unwrap() = Some(Gfx::setup().unwrap());
        let guard = GFX.lock().unwrap();
        let gfx_shared = guard.as_ref().unwrap().as_shared();
        let mut gfx = gfx_shared.lock();
        gfx.set_epd_mode(EpdMode::Quality);
        gfx.set_rotation(1);
    }
    #[cfg(target_os="linux")]
    {
        *GFX.lock().unwrap() = Some(Gfx::setup(960, 540).unwrap());
    }
    
    std::thread::spawn(|| {
        let guard = GFX.lock().unwrap();
        let gfx_shared = guard.as_ref().unwrap().as_shared();
        ui_task(gfx_shared);
    });
    *SAMPLE_TIMER_SERVICE.lock().unwrap() = Some(EspTaskTimerService::new().unwrap());
    *SAMPLE_TIMER.lock().unwrap() = Some(SAMPLE_TIMER_SERVICE.lock().unwrap().as_mut().unwrap().timer(|| sample_task())
        .expect("Failed to register sample task"));
    SAMPLE_TIMER.lock().unwrap().as_mut().unwrap().every(Duration::from_secs(30)).unwrap();
    
    // Initialize WiFi
    #[cfg(target_os="espidf")]
    let wifi = {
        let _peripherals = Peripherals::take().unwrap();
        let netif_stack = Arc::new(EspNetifStack::new()?);
        let sys_loop_stack = Arc::new(esp_idf_svc::sysloop::EspSysLoopStack::new()?);
        let default_nvs = Arc::new(esp_idf_svc::nvs::EspDefaultNvs::new()?);

        let wifi = Arc::new(Mutex::new(EspWifi::new(
            netif_stack.clone(),
            sys_loop_stack.clone(),
            default_nvs.clone(),
        )?));
        {
            log::info!("Configuring Wi-Fi");
            let mut wifi = wifi.lock().unwrap();
            wifi.set_configuration(&embedded_svc::wifi::Configuration::Client(
                embedded_svc::wifi::ClientConfiguration {
                    ssid: WIFI_AP.into(),
                    password: WIFI_PASS.into(),
                    channel: None,
                    ..Default::default()
                },
            ))?;
        }
        wifi
    };
    #[cfg(target_os="linux")]
    let wifi = Arc::new(Mutex::new(EspWifi{}));

    log::info!("Starting update task...");
    std::thread::Builder::new()
        .name("UPDATE".into())
        .stack_size(10*1024)
        .spawn(|| update_task_cloudapi(wifi))
        .expect("Failed to launch UPDATE task");
    #[cfg(target_os="linux")]
    loop { 
        Gfx::handle_sdl_event();
        std::thread::sleep(Duration::from_millis(5)); 
    }
    Ok(())
}

pub const ACCESS_TOKEN_BEARER_LENGTH: usize = "Bearer ".len() + ACCESS_TOKEN.len();

fn get_target_device() -> anyhow::Result<((Option<Device>, Option<NewestEvents>), RateLimitInfo)> {
    fetch_http_and_parse("https://api.nature.global/1/devices", |mut response| {
        let content_length = response.content_len();
        let mut target_device: Option<Device> = None;
        let mut target_device_newest_events: Option<NewestEvents> = None;
        read_devices(&mut &mut response, content_length, |device, sub_node| {
            if device.id == config::SENSOR_REMO_DEVICE_ID {
                target_device = Some(device.clone());
                if let Some(DeviceSubNode::NewestEvents(newest_events)) = sub_node {
                    target_device_newest_events = Some(newest_events.clone());
                }
            }

        })
        .map_err(|_| anyhow!("JSON parse error"))?;
        Ok((target_device, target_device_newest_events))
    })
}
fn get_target_appliance() -> anyhow::Result<((Option<Appliance>, Vec<EchonetLiteProperty, 10>), RateLimitInfo)> {
    fetch_http_and_parse("https://api.nature.global/1/appliances", |mut response| {
        let content_length = response.content_len();
        let mut target_appliance: Option<Appliance> = None;
        let mut properties = Vec::new();
        read_appliances(&mut &mut response, content_length, |appliance, sub_node| {
            //log::info!("read_appliances: {:?} {:?}", appliance, sub_node);
            if appliance.id == config::ECHONETLITE_APPLIANCE_ID {
                target_appliance = Some(appliance.clone());
                if let Some(ApplianceSubNode::EchonetLiteProperty(property)) = sub_node {
                    properties.push(property.clone());
                }
            }
        })
        .map_err(|_| anyhow!("JSON parse error"))?;
        Ok((target_appliance, properties))
    })
}
// fn get_appliances() -> anyhow::Result<Appliances> {
//     let appliances: Appliances = fetch_http("https://api.nature.global/1/appliances")?;
//     Ok(appliances)
// }

#[derive(Clone, Copy, Debug, Default)]
pub struct RateLimitInfo {
    pub limit: Option<usize>,
    pub remaining: Option<usize>,
    pub reset: Option<u64>,
}
impl RateLimitInfo {
    pub const fn new() -> Self {
        Self {
            limit: None,
            remaining: None,
            reset: None,
        }
    }
}
