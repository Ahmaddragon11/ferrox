// SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod pages;

use crate::app::{AppEvent, FerroxApp, StartupOptions};
use ferrox_core::FerroxConfig;
use ferrox_net::FastNetClient;
use mimalloc::MiMalloc;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let mut startup = StartupOptions::default();
    let mut check_update = false;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--version" | "-V" => {
                println!("Ferrox {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--check-update" => check_update = true,
            "--private" | "--incognito" => startup.private = true,
            "--turbo" => {
                if let Some(url) = args.get(index + 1) {
                    startup.url = Some(url.clone());
                    startup.turbo = true;
                    index += 1;
                } else {
                    eprintln!("--turbo requires a URL");
                }
            }
            other if !other.starts_with('-') => {
                startup.url = Some(other.to_string());
            }
            other => {
                eprintln!("Unknown option: {other}");
            }
        }
        index += 1;
    }

    if check_update {
        return run_update_check();
    }

    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut application = FerroxApp::new(proxy, startup)?;
    event_loop.run_app(&mut application)?;
    Ok(())
}

fn print_help() {
    println!(
        "Ferrox {} — fast, private, Rust-native web browser

USAGE:
    ferrox [OPTIONS] [URL]

OPTIONS:
    -h, --help            Print this help
    -V, --version         Print the installed version
        --turbo <URL>     Open a URL with the pure-Rust reader engine
        --private         Start in incognito mode
        --check-update    Check for updates and exit
",
        env!("CARGO_PKG_VERSION")
    );
}

fn run_update_check() -> Result<(), Box<dyn std::error::Error>> {
    let config = FerroxConfig::load_or_default(&FerroxConfig::default_config_path());
    let net = FastNetClient::new()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let result = runtime.block_on(ferrox_updater::check_for_update(
        net.raw_client(),
        &config.manifest_url,
        env!("CARGO_PKG_VERSION"),
        config.update_channel,
    ));

    match result {
        Ok(Some((manifest, _asset))) => {
            println!("Update available: Ferrox {}", manifest.version);
            Ok(())
        }
        Ok(None) => {
            println!("Ferrox {} is up to date", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Err(err) => {
            eprintln!("Update check failed: {err}");
            std::process::exit(2);
        }
    }
}
