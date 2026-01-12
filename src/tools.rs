use std::{error::Error, time::Duration};

use gtk4::{CssProvider, gdk::Display};
use serde::Deserialize;
use ureq::Agent;

#[derive(Deserialize)]
pub struct Info {
    pub(crate) status: String,
    availmem: f64,
    totalmem: f64,
    usedmem: f64,
    loadvg: Vec<f64>,
}

pub fn ureq_setup() -> Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(3)))
        .build()
        .into()
}

pub fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../style.css"));

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn poll_server(agent: &Agent, host: &String) -> Result<Info, Box<dyn Error>> {
    let Ok(mut res) = agent.get(format!("http://{host}:8114/status")).call() else {
        return Err(format!("Failed to connect to server {host}").into());
    };

    Ok(res.body_mut().read_json::<Info>()?)
}
