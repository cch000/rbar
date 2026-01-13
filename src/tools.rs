use std::{error::Error, time::Duration};

use gtk4::{Application, ApplicationWindow, CssProvider, gdk::Display};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
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

pub fn layer_setup(application: &Application) -> ApplicationWindow {
    let window = ApplicationWindow::new(application);
    window.init_layer_shell();
    window.set_layer(Layer::Background);

    let anchors = [
        (Edge::Left, false),
        (Edge::Right, true),
        (Edge::Top, true),
        (Edge::Bottom, false),
    ];

    for (anchor, state) in &anchors {
        window.set_anchor(*anchor, *state);
    }
    window
}

pub fn poll_server(agent: &Agent, host: &String) -> Result<Info, Box<dyn Error>> {
    let Ok(mut res) = agent.get(format!("http://{host}:8114/status")).call() else {
        return Err(format!("Failed to connect to server {host}").into());
    };

    Ok(res.body_mut().read_json::<Info>()?)
}
