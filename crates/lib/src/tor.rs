use std::{collections::HashMap, sync::Mutex};

use logger::{log::debug, Logger};
use once_cell::sync::OnceCell;
use tor::{
    ensure_runtime,
    http_client::{make_http_request, HttpMethod, HttpRequestParams},
    OwnedTorService, OwnedTorServiceBootstrapPhase, TorHiddenServiceParam, TorServiceParam,
};

use crate::ffi::bridging::{HiddenServiceResponse, HttpResponse, StartTorResponse};

use hex;
use sha2::{Digest, Sha512};
use serde::Deserialize;

static INITIALIZED: OnceCell<bool> = OnceCell::new();
static TOR_SERVICE: OnceCell<Mutex<Option<OwnedTorService>>> = OnceCell::new();

fn ensure_tor_service() -> &'static Mutex<Option<OwnedTorService>> {
    TOR_SERVICE.get_or_init(|| Mutex::new(None))
}

pub fn initialize_tor_library() -> bool {
    if INITIALIZED.get().is_some() {
        return true;
    }

    let _logger = Logger::new();

    let _ = ensure_runtime();
    let _ = ensure_tor_service();

    match INITIALIZED.set(true) {
        Ok(_) => true,
        Err(_) => false,
    }
}

pub fn init_tor_service(socks_port: f64, data_dir: String, timeout_ms: f64) -> bool {
    if INITIALIZED.get().is_none() {
        return false;
    }

    debug!(
        "Rust FFI: Initializing Tor service with parameters: socks_port={}, data_dir={}, timeout_ms={}",
        socks_port, data_dir, timeout_ms
    );

    let param = TorServiceParam {
        socks_port: Some(socks_port as u16),
        data_dir: data_dir,
        bootstrap_timeout_ms: Some(timeout_ms as u64),
    };

    debug!(
        "Rust FFI: Initializing Tor service with parameters: {:?}",
        param
    );

    match OwnedTorService::new(param) {
        Ok(service) => {
            *ensure_tor_service().lock().unwrap() = Some(service);
            debug!("Rust FFI: Tor service initialized!");
            true
        }
        Err(e) => {
            debug!("Rust FFI: Error initializing Tor service! {:?}", e);
            false
        }
    }
}

pub fn create_hidden_service(port: f64, target_port: f64) -> HiddenServiceResponse {
    internal_create_hidden_service(port, target_port, None)
}

fn internal_create_hidden_service(
    port: f64,
    target_port: f64,
    key_data: Option<[u8; 64]>,
) -> HiddenServiceResponse {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    debug!(
        "Rust FFI: Creating hidden service with parameters: port={}, target_port={}",
        port, target_port,
    );

    if let Some(service) = service_guard.as_mut() {
        let param = TorHiddenServiceParam {
            to_port: target_port as u16,
            hs_port: port as u16,
            secret_key: key_data,
        };

        debug!(
            "Rust FFI: Creating hidden service with parameters: {:?} and control port {} and control host {}",
            param.to_port,
            service.control_port.split(":").last().unwrap(),
            service.control_port.split(":").next().unwrap()
        );

        match service.create_hidden_service(param) {
            Ok(result) => {
                debug!("Rust FFI: Hidden service created {} ", result.onion_url);
                HiddenServiceResponse {
                    is_success: true,
                    onion_address: result.onion_url.to_string(),
                    control: service.control_port.trim().into(),
                }
            }
            Err(e) => {
                debug!("Rust FFI: Error creating hidden service {:?}", e);
                HiddenServiceResponse {
                    is_success: false,
                    onion_address: "".to_string(),
                    control: "".to_string(),
                }
            }
        }
    } else {
        debug!("Rust FFI: No service created");
        HiddenServiceResponse {
            is_success: false,
            onion_address: "".to_string(),
            control: "".to_string(),
        }
    }
}

#[derive(Deserialize)]
struct KeySpec {
    onion: Option<String>,
    seed_hex: Option<String>,
    pub_hex: Option<String>,
    generate: Option<bool>,
}

fn expand_seed_to_secret(seed: &[u8]) -> Option<[u8; 64]> {
    if seed.len() != 32 {
        return None;
    }

    let mut hasher = Sha512::new();
    hasher.update(seed);
    let digest = hasher.finalize();

    let mut h = [0u8; 64];
    h.copy_from_slice(&digest);

    // Clamp as in Ed25519 spec
    h[0] &= 248;
    h[31] &= 127;
    h[31] |= 64;

    Some(h)
}

pub fn start_tor_if_not_running(
    data_dir: String,
    socks_port: f64,
    target_port: f64,
    timeout_ms: f64,
    keys_json: String,
) -> StartTorResponse {
    if !initialize_tor_library() {
        return StartTorResponse {
            is_success: false,
            onion_address: String::new(),
            control: String::new(),
            error_message: "Failed to initialize Tor library".to_string(),
            onion_addresses_json: String::new(),
        };
    }

    let status = get_service_status();

    if status == 2.0 {
        debug!(
            "Rust FFI: Tor service needs initialization. Status: {}",
            status
        );

        if !init_tor_service(socks_port, data_dir, timeout_ms) {
            return StartTorResponse {
                is_success: false,
                onion_address: String::new(),
                control: String::new(),
                error_message: "Failed to initialize Tor service".to_string(),
                onion_addresses_json: String::new(),
            };
        }
    } else {
        debug!(
            "Rust FFI: Tor service already initialized. Status: {}",
            status
        );
    }

    let mut onion_addresses: Vec<String> = Vec::new();
    let mut control_port: String = String::new();

    if !keys_json.is_empty() {
        match serde_json::from_str::<Vec<KeySpec>>(&keys_json) {
            Ok(specs) => {
                for spec in specs.into_iter() {
                    let generate = spec.generate.unwrap_or(true);

                    let key_bytes: Option<[u8; 64]> = if generate {
                        None
                    } else if let Some(seed_hex) = spec.seed_hex.as_ref() {
                        match hex::decode(seed_hex) {
                            Ok(seed) => expand_seed_to_secret(&seed),
                            Err(_) => None,
                        }
                    } else {
                        None
                    };

                    let hs_response = internal_create_hidden_service(socks_port, target_port, key_bytes);
                    if hs_response.is_success {
                        if control_port.is_empty() {
                            control_port = hs_response.control.clone();
                        }
                        onion_addresses.push(hs_response.onion_address);
                    }
                }
            }
            Err(e) => {
                debug!(
                    "Rust FFI: Failed to parse keys_json for start_tor_if_not_running: {:?}",
                    e
                );
            }
        }
    }

    if onion_addresses.is_empty() {
        let hs_response = internal_create_hidden_service(socks_port, target_port, None);

        let is_success = hs_response.is_success;
        let onion_address = if is_success {
            hs_response.onion_address.clone()
        } else {
            String::new()
        };
        let control = if is_success {
            hs_response.control.clone()
        } else {
            String::new()
        };

        let onion_list = if is_success {
            vec![onion_address.clone()]
        } else {
            Vec::new()
        };

        let onion_addresses_json = serde_json::to_string(&onion_list).unwrap_or_else(|_| "[]".to_string());

        return StartTorResponse {
            is_success,
            onion_address,
            control,
            error_message: if is_success {
                String::new()
            } else {
                "Failed to create hidden service".to_string()
            },
            onion_addresses_json,
        };
    }

    let onion_address = onion_addresses[0].clone();
    let onion_addresses_json = serde_json::to_string(&onion_addresses).unwrap_or_else(|_| "[]".to_string());

    StartTorResponse {
        is_success: true,
        onion_address,
        control: control_port,
        error_message: String::new(),
        onion_addresses_json,
    }
}

pub fn get_service_status() -> f64 {
    let service_guard = ensure_tor_service().lock().unwrap();

    match &*service_guard {
        Some(service) => match service.get_status() {
            Ok(OwnedTorServiceBootstrapPhase::Done) => 1.0,
            Ok(_) => 0.0,
            Err(_) => 2.0,
        },
        None => 2.0,
    }
}

pub fn delete_hidden_service(address: String) -> bool {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    if let Some(service) = service_guard.as_mut() {
        service.delete_hidden_service(address).is_ok()
    } else {
        false
    }
}

pub fn shutdown_service() -> bool {
    let mut service_guard = ensure_tor_service().lock().unwrap();

    if let Some(mut service) = service_guard.take() {
        service.shutdown().is_ok()
    } else {
        false
    }
}

fn make_tor_http_request(
    url: String,
    method: HttpMethod,
    headers_json: String,
    body: String,
    timeout_ms: u64,
) -> HttpResponse {
    if INITIALIZED.get().is_none() {
        return HttpResponse {
            status_code: 0.0,
            body: String::new(),
            error: "Tor library not initialized".to_string(),
        };
    }

    debug!(
        "http request params: {:?} {:?} {:?} {}",
        url, headers_json, body, timeout_ms
    );

    // Parse headers JSON if provided
    let headers: Option<HashMap<String, String>> = if !headers_json.is_empty() {
        match serde_json::from_str(&headers_json) {
            Ok(h) => Some(h),
            Err(_) => {
                return HttpResponse {
                    status_code: 0.0,
                    body: String::new(),
                    error: "Invalid headers JSON".to_string(),
                };
            }
        }
    } else {
        None
    };

    // Create request params
    let params = HttpRequestParams {
        url,
        method,
        headers,
        body: if body.is_empty() { None } else { Some(body) },
        timeout_ms: Some(timeout_ms),
    };

    // Get socks proxy address from the running Tor service
    let service_guard = ensure_tor_service().lock().unwrap();
    let socks_port = match &*service_guard {
        Some(service) => service.socks_port,
        None => {
            return HttpResponse {
                status_code: 0.0,
                body: String::new(),
                error: "Tor service not running".to_string(),
            };
        }
    };

    debug!("socks port: {}", socks_port);

    // Make the HTTP request
    let socks_proxy = format!("127.0.0.1:{}", socks_port);
    match make_http_request(params, socks_proxy) {
        Ok(response) => {
            debug!("http response: {:?}", response);
            return HttpResponse {
                status_code: response.status_code as f64,
                body: response.body,
                error: match response.error {
                    Some(err) => err,
                    None => String::new(),
                },
            };
        }
        Err(e) => {
            debug!("http error: {:?}", e);
            return HttpResponse {
                status_code: 0.0,
                body: String::new(),
                error: format!("Error making HTTP request: {:?}", e),
            };
        }
    }
}

pub fn http_get(url: String, headers_json: String, timeout_ms: f64) -> HttpResponse {
    make_tor_http_request(
        url,
        HttpMethod::GET,
        headers_json,
        String::new(), // No body for GET
        timeout_ms as u64,
    )
}

pub fn http_post(url: String, body: String, headers_json: String, timeout_ms: f64) -> HttpResponse {
    make_tor_http_request(url, HttpMethod::POST, headers_json, body, timeout_ms as u64)
}

pub fn http_put(url: String, body: String, headers_json: String, timeout_ms: f64) -> HttpResponse {
    make_tor_http_request(url, HttpMethod::PUT, headers_json, body, timeout_ms as u64)
}

pub fn http_delete(url: String, headers_json: String, timeout_ms: f64) -> HttpResponse {
    make_tor_http_request(
        url,
        HttpMethod::DELETE,
        headers_json,
        String::new(), // Usually no body for DELETE
        timeout_ms as u64,
    )
}
