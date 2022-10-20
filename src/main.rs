use std::{sync::Arc, time::Duration};

use embedded_svc::{wifi::{ClientConnectionStatus, ClientIpStatus, ClientStatus, Wifi}, timer::{TimerService, PeriodicTimer}};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{netif::EspNetifStack, wifi::EspWifi, timer::{EspTimerService, EspTaskTimerService, EspTimer}};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use rand::prelude::*;

use lgfx::{self, ColorRgb332};

use anyhow::bail;

const LOGO_PNG: &[u8; 9278] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/rust-logo-512x512-blk_white.png"
));

use lgfx::{DrawImage, DrawPrimitives, Gfx};

use crate::lgfx::{EpdMode, DrawChars, FontManupulation, LgfxDisplay};

const WIFI_AP: &str = env!("WIFI_AP");
const WIFI_PASS: &str = env!("WIFI_PASS");

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

type Timestamp = std::time::SystemTime;

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

fn update_task() -> ! {
    let mut rng = rand::thread_rng();
    loop {
        let record = SensorRecord {
            ambient_temperature: rng.gen_range(0.0..40.0),
            relative_humidity: rng.gen_range(0.0..=100.0),
            ambient_luminous_level: rng.gen_range(0.0..=100.0),
            instant_power_usage: rng.gen_range(0.0..2000.0),
        };
        let timestamp = std::time::SystemTime::now();
        log::info!("update task: {:?} {:?}", record, timestamp);
        *LAST_RECORD.lock().unwrap() = Some((record, timestamp));
        std::thread::sleep(Duration::from_secs(30));
    }
}

fn sample_task() {
    let timestamp =  std::time::SystemTime::now();
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
        let latest = sensor_records.latest();
        if let Some((record, timestamp)) = latest {
            use std::fmt::Write;
            write!(&mut min_temperature_str, "{:6.1}", min.ambient_temperature);
            write!(&mut cur_temperature_str, "{:6.1}", record.ambient_temperature);
            write!(&mut max_temperature_str, "{:6.1}", max.ambient_temperature);
            write!(&mut min_humidity_str, "{:6.1}", min.relative_humidity);
            write!(&mut cur_humidity_str, "{:6.1}", record.relative_humidity);
            write!(&mut max_humidity_str, "{:6.1}", max.relative_humidity);
            write!(&mut min_power_str, "{:6.1}", min.instant_power_usage);
            write!(&mut cur_power_str, "{:6.1}", record.instant_power_usage);
            write!(&mut max_power_str, "{:6.1}", max.instant_power_usage);
            write!(&mut timestamp_str, "{:?}", timestamp);
        } else {
            use std::fmt::Write;
            min_temperature_str.write_str("--");
            cur_temperature_str.write_str("--");
            max_temperature_str.write_str("--");
            min_humidity_str.write_str("--");
            cur_humidity_str.write_str("--");
            max_humidity_str.write_str("--");
            min_power_str.write_str("--");
            cur_power_str.write_str("--");
            max_power_str.write_str("--");
            timestamp_str.write_str("--");
        }
        {
            let mut guard = gfx.lock_without_auto_update();
            let foreground = ColorRgb332::new(0xff);
            let background = ColorRgb332::new(0x00);
            guard.set_font(lgfx::fonts::FreeMono24pt7b);
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
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("Hello, world!");
    *GFX.lock().unwrap() = Some(Gfx::setup().unwrap());
    {
        let guard = GFX.lock().unwrap();
        let gfx_shared = guard.as_ref().unwrap().as_shared();
        let mut gfx = gfx_shared.lock();
        gfx.set_epd_mode(EpdMode::Quality);
        gfx.set_rotation(1);
    }
    
    std::thread::spawn(|| {
        let guard = GFX.lock().unwrap();
        let gfx_shared = guard.as_ref().unwrap().as_shared();
        ui_task(gfx_shared);
    });
    std::thread::spawn(|| {
        update_task();
    });
    *SAMPLE_TIMER_SERVICE.lock().unwrap() = Some(EspTaskTimerService::new().unwrap());
    *SAMPLE_TIMER.lock().unwrap() = Some(SAMPLE_TIMER_SERVICE.lock().unwrap().as_mut().unwrap().timer(|| sample_task())
        .expect("Failed to register sample task"));
    SAMPLE_TIMER.lock().unwrap().as_mut().unwrap().every(Duration::from_secs(30)).unwrap();
    
    let peripherals = Peripherals::take().unwrap();
    let netif_stack = Arc::new(EspNetifStack::new()?);
    let sys_loop_stack = Arc::new(esp_idf_svc::sysloop::EspSysLoopStack::new()?);
    let default_nvs = Arc::new(esp_idf_svc::nvs::EspDefaultNvs::new()?);

    let mut wifi = Box::new(EspWifi::new(
        netif_stack.clone(),
        sys_loop_stack.clone(),
        default_nvs.clone(),
    )?);

    wifi.set_configuration(&embedded_svc::wifi::Configuration::Client(
        embedded_svc::wifi::ClientConfiguration {
            ssid: WIFI_AP.into(),
            password: WIFI_PASS.into(),
            channel: None,
            ..Default::default()
        },
    ))?;
    log::info!("Configuring Wi-Fi");
    wifi.wait_status_with_timeout(Duration::from_secs(10), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected wifi status: {:?}", e))?;

    let status = wifi.get_status();
    if let embedded_svc::wifi::Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        _,
    ) = status
    {
        log::info!("WiFi connected: ip = {:?}", ip_settings);
        //ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wi-Fi Status: {:?}", status);
    }

    access_http()?;

    Ok(())
}

const ACCESS_TOKEN: &str = concat!(
    "Bearer ",
    "mW6DeFliMYI--hmOU77QL3adkhRGbmkRcywop8w_bAQ.MEOXdyjjuQ-otVYBQc85PnkNLTouLv_gKL3YxXE2WnM"
);

fn access_http() -> anyhow::Result<()> {
    use embedded_svc::http::{self, client::*, status, Headers, Status};
    use embedded_svc::io;
    use esp_idf_svc::http::client::*;

    let url = String::from("https://api.nature.global/1/appliances");
    let mut client = EspHttpClient::new(&EspHttpClientConfiguration {
        crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut request = client.get(&url)?;
    request.set_header("Authorization", ACCESS_TOKEN);
    let mut response = request.submit()?;
    let mut body = [0u8; 3072];
    let (body, _) = io::read_max(response.reader(), &mut body)?;
    log::info!("Body:\n{:?}", String::from_utf8_lossy(body).into_owned(),);

    Ok(())

    // let resp = attohttpc::get("http://example.com/").send()?;
    // if resp.is_success() {
    //     log::info!("response: {}", resp.text()?);
    // } else {
    //     log::error!("get error: {:?}", resp.error_for_status()?);
    // }
    // Ok(())
}
