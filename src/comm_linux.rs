use std::{fmt::Write, io::Read, time::{Duration, Instant}, thread::JoinHandle, sync::{Arc, Mutex, Condvar}};

use crate::{RateLimitInfo, config::ACCESS_TOKEN};

pub const MAX_ACCESS_TOKEN_LEN: usize = 128;
pub const ACCESS_TOKEN_BEARER_LENGTH: usize = "Bearer ".len() + MAX_ACCESS_TOKEN_LEN;

/// Dummy implementation of EspWifi
pub struct EspWifi {}

#[derive(Debug)]
pub enum EspError {}

#[derive(Clone, Copy, Debug)]
pub struct IpSettings {}
#[derive(Debug)]
pub enum ClientIpStatus {
    Done(IpSettings),
}
#[derive(Debug)]
pub enum ClientConnectionStatus {
    Connected(ClientIpStatus),
    Disconnected,
}
#[derive(Debug)]
pub enum ClientStatus {
    Started(ClientConnectionStatus),
}
impl ClientStatus {
    pub fn is_transitional(&self) -> bool { false }
}

impl EspWifi {
    pub fn start(&self) -> Result<(), EspError> { Ok(()) }
    pub fn connect(&self) -> Result<(), EspError> { Ok(()) }
    pub fn is_started(&self) -> anyhow::Result<bool> { Ok(true) }
    pub fn is_connected(&self) -> anyhow::Result<bool> { Ok(true) }
}

pub struct WifiWait {}
impl WifiWait {
    pub fn wait_with_timeout<F: Fn() -> bool>(&self, duration: Duration, predicate: F) -> bool { predicate() }
}

pub struct HttpResponse<'a> {
    response: &'a mut reqwest::blocking::Response,
}

impl<'a> HttpResponse<'a> {
    pub fn content_len(&self) -> Option<usize> {
        self.response.content_length().map(|v| v as usize)
    }
}

impl<'a> embedded_io::Io for HttpResponse<'a> {
    type Error = embedded_io::ErrorKind;
}
impl<'a> embedded_io::blocking::Read for HttpResponse<'a> {
    
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.response.read(buf).map_err(|_| embedded_io::ErrorKind::Other)
    }
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<(), embedded_io::blocking::ReadExactError<Self::Error>> {
        self.response.read_exact(buf).map_err(|_| embedded_io::blocking::ReadExactError::Other(embedded_io::ErrorKind::Other))
    }
}

impl<'a> From<&'a mut reqwest::blocking::Response> for HttpResponse<'a> {
    fn from(response: &'a mut reqwest::blocking::Response) -> Self {
        Self {
            response,
        }
    }
}

pub fn fetch_http_and_parse<F, ParserResult>(url: &str, access_token: &str, mut response_parser: F) -> anyhow::Result<(ParserResult, RateLimitInfo)> 
    where F: for <'a> FnMut(HttpResponse<'a>) -> anyhow::Result<ParserResult>
{
    let mut access_token = heapless::String::<ACCESS_TOKEN_BEARER_LENGTH>::new();
    access_token.write_str("Bearer ").unwrap();
    access_token.write_str(ACCESS_TOKEN).unwrap();
    let mut client = reqwest::blocking::Client::new();
    let mut response = client.get(url)
        .header("Authorization", access_token.as_str())
        .send()?;

    //< x-rate-limit-limit: 30
    //< x-rate-limit-remaining: 25
    //< x-rate-limit-reset: 1667922300
    //< x-xss-protection: 1; mode=block
    let rate_limit = RateLimitInfo {
        limit: response.headers().get("x-rate-limit-limit").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()),
        remaining: response.headers().get("x-rate-limit-remaining").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()),
        reset: response.headers().get("x-rate-limit-reset").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()),
    };
    let result = response_parser(HttpResponse::from(&mut response))?;
    Ok((result, rate_limit))
}

pub struct EspTaskTimerService{}
impl EspTaskTimerService {
    pub fn new() -> Result<Self, ()> { Ok(Self {}) }
    pub fn timer(&self, callback: impl FnMut() + Send + 'static) -> Result<EspTimer, ()> {
        Ok(EspTimer {
            callback: Arc::new(Mutex::new(callback)),
            thread: None,
            exit: Arc::new(Mutex::new(false)),
        })
    }
}

pub struct EspTimer {
    callback: Arc<Mutex<dyn FnMut() + Send + 'static>>,
    thread: Option<JoinHandle<()>>,
    exit: Arc<Mutex<bool>>,
}

impl EspTimer {
    pub fn every(&mut self, interval: Duration) -> Result<(), ()> {
        let mut thread = None;
        std::mem::swap(&mut thread, &mut self.thread);
        if let Some(thread) = thread {
            *self.exit.lock().unwrap() = true;
            thread.join().map_err(|_| ())?;
        }
        *self.exit.lock().unwrap() = false;

        let exit = self.exit.clone();
        let callback = self.callback.clone();
        let thread = std::thread::spawn(move || {
            let mut last_time = Instant::now();
            loop {
                if *exit.lock().unwrap() {
                    break;
                }
                let now = Instant::now();
                if now.duration_since(last_time) < interval {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                last_time = now;
                callback.lock().unwrap()();
            }
        });
        self.thread = Some(thread);
        Ok(())
    }
}

impl Drop for EspTimer {
    fn drop(&mut self) {
        let mut thread = None;
        std::mem::swap(&mut thread, &mut self.thread);
        if let Some(thread) = thread {
            *self.exit.lock().unwrap() = true;
            thread.join().ok();
        }
    }
}