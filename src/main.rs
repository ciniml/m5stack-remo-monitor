
#[cfg(target_os="espidf")]
pub mod comm_esp;
#[cfg(target_os="espidf")]
mod espidf_imports {
    pub use super::comm_esp::*;
    pub use embedded_svc::{wifi::*, timer::*, http::client::Response};
    pub use esp_idf_hal::prelude::Peripherals;
    pub use esp_idf_svc::{netif::*, wifi::*, timer::*, nvs::*, eventloop::EspSystemEventLoop};
    pub use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
    pub use embedded_svc::http::Headers;
}
#[cfg(target_os="espidf")]
use espidf_imports::*;

#[cfg(target_os="linux")]
mod comm_linux;
#[cfg(target_os="linux")]
use comm_linux::*;

use std::{sync::{Arc, Mutex}, time::Duration, str::FromStr, fmt::Write, ffi::CStr};

use anyhow::anyhow;
use embedded_io::blocking::Read;

use rand::prelude::*;

use lgfx::{self, ColorRgb332, DrawString, textdatum_top_left};
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

use fuga_remo_api::{Device, read_devices, DeviceSubNode, NewestEvents, EchonetLiteProperty, Appliance, read_appliances, ApplianceSubNode, ParserOptions};

mod config;
use config::*;

mod chart;
use chart::Chart;

#[derive(Default, Debug)]
struct Config {
    wifi_ssid: heapless::String<32>,
    wifi_password: heapless::String<64>,
    device_id: Uuid,
    appliance_id: Uuid,
    access_token: heapless::String<128>,
}

static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

#[cfg(target_os="espidf")]
fn init_config() {
    let mut config = Config::default();
    let nvs_partition = EspDefaultNvsPartition::take().unwrap();
    {
        let nvs_partition = nvs_partition.clone();
        let nvs = EspDefaultNvs::new(nvs_partition, "wifi", false).unwrap();
        let mut buffer = [0u8; 128];
        config.wifi_ssid = heapless::String::from_str(nvs.get_str("ssid", &mut buffer).unwrap().unwrap_or("")).unwrap();
        config.wifi_password = heapless::String::from_str(nvs.get_str("pass", &mut buffer).unwrap().unwrap_or("")).unwrap();
    }
    {
        let nvs_partition = nvs_partition.clone();
        let nvs = EspDefaultNvs::new(nvs_partition, "device", false).unwrap();
        let mut buffer = [0u8; 128];
        config.device_id = Uuid::from_str(nvs.get_str("device_id", &mut buffer).unwrap().unwrap_or("")).unwrap_or_default();
        config.appliance_id = Uuid::from_str(nvs.get_str("appliance_id", &mut buffer).unwrap().unwrap_or("")).unwrap_or_default();
        config.access_token = heapless::String::from_str(nvs.get_str("access_token", &mut buffer).unwrap().unwrap_or("")).unwrap();
    }
    log::info!("init_config {:?}", config);
    *CONFIG.lock().unwrap() = Some(config);
}
#[cfg(target_os="linux")]
fn init_config() {
    let config = Config {
        wifi_ssid: heapless::String::from_str(config::WIFI_AP).unwrap(),
        wifi_password: heapless::String::from_str(config::WIFI_PASS).unwrap(),
        device_id: config::SENSOR_REMO_DEVICE_ID,
        appliance_id: config::ECHONETLITE_APPLIANCE_ID,
        access_token: heapless::String::from_str(config::ACCESS_TOKEN).unwrap(),
    };
    log::info!("init_config {:?}", config);
    *CONFIG.lock().unwrap() = Some(config);
}

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
static IS_WIFI_CONNECTED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

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
        std::thread::sleep(Duration::from_secs(1));
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

fn update_task_cloudapi(wifi: Arc<Mutex<EspWifi>>, wifi_wait: WifiWait) -> ! {
    let mut prev_has_network_connection = false;

    loop {
        let has_network_connection = {
            let mut wifi = wifi.lock().unwrap();
            if wifi.is_connected().unwrap_or(false) {
                true
            } else {
                let is_started = if !wifi.is_started().unwrap_or(false) {
                    log::info!("Starting WiFi...");
                    match wifi.start() {
                        Ok(_) => {
                            log::info!("Waiting WiFi starts...");
                            wifi_wait.wait_with_timeout(Duration::from_secs(10), || wifi.is_started().unwrap_or(false))
                        },
                        Err(err) => {
                            log::error!("Failed to start WiFi - {:?}", err);
                            false
                        },
                    }
                } else {
                    true
                };
                if is_started {
                    log::info!("Waiting WiFi gets connected...");
                    if !wifi.is_connected().unwrap_or(false) {
                        log::info!("Connectting WiFi...");
                        match wifi.connect() {
                            Ok(_) => {
                                log::info!("Waiting WiFi connection...");
                                wifi_wait.wait_with_timeout(Duration::from_secs(10), || wifi.is_connected().unwrap_or(false))
                            },
                            Err(err) => {
                                log::error!("Failed to connect WiFi - {:?}", err);
                                false
                            },
                        }
                    } else {
                        true
                    }
                } else {
                    false
                }
            }
        };
        *IS_WIFI_CONNECTED.lock().unwrap() = has_network_connection;
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
        let mut wifi_connection_str = heapless::String::<16>::new();

        let latest = sensor_records.latest();
        if let Some((record, timestamp)) = latest {
            write!(&mut min_temperature_str, "{:4.1}", min.ambient_temperature).ok();
            write!(&mut cur_temperature_str, "{:4.1}", record.ambient_temperature).ok();
            write!(&mut max_temperature_str, "{:4.1}", max.ambient_temperature).ok();
            write!(&mut min_humidity_str, "{:4.1}", min.relative_humidity).ok();
            write!(&mut cur_humidity_str, "{:4.1}", record.relative_humidity).ok();
            write!(&mut max_humidity_str, "{:4.1}", max.relative_humidity).ok();
            write!(&mut min_power_str, "{:5.0}", min.instant_power_usage).ok();
            write!(&mut cur_power_str, "{:5.0}", record.instant_power_usage).ok();
            write!(&mut max_power_str, "{:5.0}", max.instant_power_usage).ok();
            write!(&mut timestamp_str, "{:?}", timestamp).ok();
        } else {
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

        // Draw top bar
        {
            write!(&mut rate_limit_str, "API: ").ok();
            if let Some(limit) = rate_limit.limit {
                if let Some(remaining) = rate_limit.remaining {
                    write!(&mut rate_limit_str, "{}/{}", remaining, limit).ok();
                }
            }
            // if let Some(reset) = rate_limit.reset {
            //     let timestamp = Timestamp::from_utc(chrono::NaiveDateTime::from_timestamp(reset as i64, 0), chrono::Utc);
            //     write!(&mut rate_limit_str, " reset at: {}", timestamp.format("%Y-%m-%d %H:%M:%S"));
            // }
            write!(&mut wifi_connection_str, "WIFI: ").ok();
            write!(&mut wifi_connection_str, "{}", if *IS_WIFI_CONNECTED.lock().unwrap() { "OK" } else { "NC" } ).ok();
        }
        {
            let mut guard = gfx.lock_without_auto_update();
            let foreground = ColorRgb332::new(0xff);
            let background = ColorRgb332::new(0x00);
            guard.set_font(lgfx::fonts::FreeMono24pt7b).ok();
            let font_height = guard.font_height();
            let line_height = font_height * 9 / 8;
            guard.clear(lgfx::ColorRgb332::new(0xff));
            let screen_width = 960;
            let screen_height = 540;
            let chart_left = 300;
            let chart_width = screen_width - chart_left;
            // Draw TOP BAR
            {
                guard.fill_rect(0, 0, screen_width, font_height, background);
                guard.draw_string(&rate_limit_str, 0, 0, background, foreground, 0.75, 0.75, textdatum_top_left);
                guard.draw_string(&rate_limit_str, 0, 0, background, foreground, 0.75, 0.75, textdatum_top_left);
                guard.draw_string(&wifi_connection_str, 300, 0, background, foreground, 0.75, 0.75, textdatum_top_left);
            }
            let chart_height = (540 - line_height) / 3;
            let value_margin_left = 20;
            let value_width = 200;
            {
                let mut y_offset = font_height;
                let mut record_iter = sensor_records.records.iter();
                Chart::new(chart_width, chart_height, background, foreground)
                    .draw(&mut guard, chart_left, y_offset, sensor_records.records.len(), min.ambient_temperature, max.ambient_temperature, move |_| record_iter.next().map(|item| item.ambient_temperature) )
                    .ok();
                guard.draw_string("Temperature:", 0, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
                y_offset += line_height;
                guard.draw_string(&cur_temperature_str, value_margin_left, y_offset, foreground, background, 1.0, 1.0, textdatum_top_left);
                y_offset += line_height;
                guard.draw_string(&min_temperature_str, value_margin_left, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
                y_offset += line_height * 3 / 4;
                guard.draw_string(&max_temperature_str, value_margin_left, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);

            }
            {
                let mut y_offset = font_height + chart_height;
                let mut record_iter = sensor_records.records.iter();
                Chart::new(chart_width, chart_height, background, foreground)
                    .draw(&mut guard, chart_left, y_offset, sensor_records.records.len(), min.relative_humidity, max.relative_humidity, move |_| record_iter.next().map(|item| item.relative_humidity) )
                    .ok();
                guard.draw_string("Humidity:", 0, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
                y_offset += line_height;
                guard.draw_string(&cur_humidity_str, value_margin_left, y_offset, foreground, background, 1.0, 1.0, textdatum_top_left);
                y_offset += line_height;
                guard.draw_string(&max_humidity_str, value_margin_left, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
                y_offset += line_height * 3 / 4;
                guard.draw_string(&min_humidity_str, value_margin_left, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
            }
            {
                let mut y_offset = font_height + chart_height * 2;
                let mut record_iter = sensor_records.records.iter();
                Chart::new(chart_width, chart_height, background, foreground)
                    .draw(&mut guard, chart_left, y_offset, sensor_records.records.len(), min.instant_power_usage, max.instant_power_usage, move |_| record_iter.next().map(|item| item.instant_power_usage) )
                    .ok();
                guard.draw_string("Power:", 0, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
                y_offset += line_height;
                guard.draw_string(&cur_power_str, value_margin_left, y_offset, foreground, background, 1.0, 1.0, textdatum_top_left);
                y_offset += line_height;
                guard.draw_string(&max_power_str, value_margin_left, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);    
                y_offset += line_height * 3 / 4;
                guard.draw_string(&min_power_str, value_margin_left, y_offset, foreground, background, 0.75, 0.75, textdatum_top_left);
            }

        }
        std::thread::sleep(Duration::from_secs(30));
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
        env_logger::init();
        *GFX.lock().unwrap() = Some(Gfx::setup(960, 540).unwrap());
    }

    // Initialize configuration.
    init_config();
    log::info!("CONFIG: {:?}", CONFIG.lock().unwrap().as_ref().unwrap());

    std::thread::Builder::new().stack_size(8192).spawn(|| {
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
    let (wifi, wifi_wait) = {
        let peripherals = Peripherals::take().unwrap();
        let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take()?;

        let wifi = Arc::new(Mutex::new(EspWifi::new(
            peripherals.modem,
            sysloop.clone(),
            None,
        )?));
        {
            log::info!("Configuring Wi-Fi");
            let (ssid, pass) = {
                let guard = CONFIG.lock().unwrap();
                let config = guard.as_ref().unwrap();
                (config.wifi_ssid.clone(), config.wifi_password.clone())
            };
            let mut wifi = wifi.lock().unwrap();
            wifi.set_configuration(&embedded_svc::wifi::Configuration::Client(
                embedded_svc::wifi::ClientConfiguration {
                    ssid: ssid.into(),
                    password: pass.into(),
                    channel: None,
                    ..Default::default()
                },
            ))?;
        }

        let wifi_wait = WifiWait::new(&sysloop)?;
        (wifi, wifi_wait)
    };
    #[cfg(target_os="linux")]
    let (wifi, wifi_wait) = {
        (Arc::new(Mutex::new(EspWifi{})), WifiWait {})
    };
    log::info!("Starting update task...");
    std::thread::Builder::new()
        .name("UPDATE".into())
        .stack_size(15*1024)
        .spawn(|| update_task_cloudapi(wifi, wifi_wait))
        //.spawn(|| update_task_random(wifi))
        .expect("Failed to launch UPDATE task");
    #[cfg(target_os="linux")]
    loop { 
        Gfx::handle_sdl_event();
        std::thread::sleep(Duration::from_millis(5)); 
    }
    Ok(())
}

fn get_target_device() -> anyhow::Result<((Option<Device>, Option<NewestEvents>), RateLimitInfo)> {
    let sensor_remo_device_id = CONFIG.lock().unwrap().as_ref().unwrap().device_id;
    let access_token = CONFIG.lock().unwrap().as_ref().unwrap().access_token.clone();
    fetch_http_and_parse("https://api.nature.global/1/devices", access_token.as_str(),|mut response| {
        let content_length = response.content_len().map(|n| n as usize);
        let mut target_device: Option<Device> = None;
        let mut target_device_newest_events: Option<NewestEvents> = None;
        read_devices(&mut &mut response, content_length, &ParserOptions::default(), |device, sub_node| {
            if device.id == sensor_remo_device_id {
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
    let echonetlite_appliance_id = CONFIG.lock().unwrap().as_ref().unwrap().appliance_id;
    let access_token = CONFIG.lock().unwrap().as_ref().unwrap().access_token.clone();
    fetch_http_and_parse("https://api.nature.global/1/appliances", access_token.as_str(),|mut response| {
        let content_length = response.content_len().map(|n| n as usize);
        let mut target_appliance: Option<Appliance> = None;
        let mut properties = Vec::new();
        read_appliances(&mut &mut response, content_length, &ParserOptions::default(), |appliance, sub_node| {
            //log::info!("read_appliances: {:?} {:?}", appliance, sub_node);
            if appliance.id == echonetlite_appliance_id {
                target_appliance = Some(appliance.clone());
                if let Some(ApplianceSubNode::EchonetLiteProperty(property)) = sub_node {
                    properties.push(property.clone());
                }
            }
        })
        .map_err(|err| anyhow!("JSON parse error - {:?}", err))?;
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
