use std::{sync::Arc, time::Duration};

use embedded_svc::wifi::{Wifi, ClientStatus, ClientConnectionStatus, ClientIpStatus};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{netif::EspNetifStack, wifi::EspWifi};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
mod lgfx;

use anyhow::bail;

const LOGO_PNG: &[u8; 9278] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/rust-logo-512x512-blk_white.png"
));

use lgfx::{DrawImage, DrawPrimitives, Gfx};

use crate::lgfx::{DrawChars, FontManupulation};

const WIFI_AP: &str = env!("WIFI_AP");
const WIFI_PASS: &str = env!("WIFI_PASS");

fn main() -> anyhow::Result<()> {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    println!("Hello, world!");
    let gfx = Gfx::setup().unwrap();
    gfx.fill_rect(0, 0, 32, 32, lgfx::ColorRgb332::new(0));
    gfx.draw_png(LOGO_PNG)
        .postion(32, 0)
        .scale(0.8, 0.0)
        .execute();
    gfx.set_font(lgfx::LgfxFontId::Font4).unwrap();
    gfx.set_text_size(2.0, 2.0);
    gfx.draw_chars(
        "Hello, Rust!",
        0,
        640,
        lgfx::ColorRgb332::new(0),
        lgfx::ColorRgb332::new(0xff),
        1.0,
        1.0,
    );
    gfx.draw_line(100, 600, 200, 700, lgfx::ColorRgb332::new(0));
    let sprite = gfx.create_sprite(64, 64).unwrap();
    sprite.clear(lgfx::ColorRgb332::new(0xff));
    sprite.fill_rect(0, 0, 32, 32, lgfx::ColorRgb332::new(0));
    sprite.fill_rect(32, 32, 32, 32, lgfx::ColorRgb332::new(0));
    sprite.push_sprite(0, 512);
    sprite.push_sprite(512 - 64, 512);

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
        }
    ))?;
    log::info!("Configuring Wi-Fi");
    wifi.wait_status_with_timeout(Duration::from_secs(10), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected wifi status: {:?}", e))?;
    
    let status = wifi.get_status();
    if let embedded_svc::wifi::Status(ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))), _) = status {
        log::info!("WiFi connected: ip = {:?}", ip_settings);
        //ping(&ip_settings)?;
    } else {
        bail!("Unexpected Wi-Fi Status: {:?}", status);
    }

    access_http()?;

    Ok(())
}

const ACCESS_TOKEN: &str = concat!("Bearer ", "mW6DeFliMYI--hmOU77QL3adkhRGbmkRcywop8w_bAQ.MEOXdyjjuQ-otVYBQc85PnkNLTouLv_gKL3YxXE2WnM");

fn access_http() -> anyhow::Result<()> {
    use embedded_svc::http::{self, client::*, status, Headers, Status};
    use embedded_svc::io;
    use esp_idf_svc::http::client::*;

    let url = String::from("https://api.nature.global/1/appliances");
    let mut client = EspHttpClient::new(&EspHttpClientConfiguration { crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),..Default::default() })?;
    let mut request = client
        .get(&url)?;
    request.set_header("Authorization", ACCESS_TOKEN);
    let mut response = request.submit()?;
    let mut body = [0u8; 3072];
    let (body, _) = io::read_max(response.reader(), &mut body)?;
    log::info!(
        "Body:\n{:?}",
        String::from_utf8_lossy(body).into_owned(),
    );

    Ok(())

    // let resp = attohttpc::get("http://example.com/").send()?;
    // if resp.is_success() {
    //     log::info!("response: {}", resp.text()?);
    // } else {
    //     log::error!("get error: {:?}", resp.error_for_status()?);
    // }
    // Ok(())
}