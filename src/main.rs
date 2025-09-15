use gio::glib::{self, ExitCode};
use gtk4::gdk::Display;
use gtk4::{Application, ApplicationWindow, Label};
use gtk4::{CssProvider, prelude::*};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use serde::Deserialize;
use std::error::Error;

#[derive(Clone)]
struct Host {
    label: Label,
    hostname: String,
}

#[derive(Deserialize)]
struct Info {
    status: String,
    availmem: f64,
    totalmem: f64,
    usedmem: f64,
    loadvg: Vec<f64>,
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../style.css"));

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn poll_server(hostname: &str) -> Result<Info, Box<dyn Error>> {
    let Ok(res) = reqwest::blocking::get(format!("http://{hostname}:8114/status")) else {
        return Err(format!("Failed to connect to server {hostname}").into());
    };

    Ok(res.json::<Info>()?)
}

fn update_label(label: &Label, hostname: &str) {
    match poll_server(hostname) {
        Ok(info) => {
            if info.status == "running" {
                label.set_css_classes(&["hostname-running"]);
            } else {
                label.set_css_classes(&["hostname-not-running"]);
            }
        }
        Err(_) => label.set_css_classes(&["hostname-unreachable"]),
    }
}

fn activate_with_hostnames(application: &Application, hostnames: &[String]) {
    let window = ApplicationWindow::new(application);

    window.init_layer_shell();
    window.set_layer(Layer::Background);
    window.set_default_height(28);

    let anchors = [
        (Edge::Left, false),
        (Edge::Right, true),
        (Edge::Top, true),
        (Edge::Bottom, false),
    ];

    for (anchor, state) in &anchors {
        window.set_anchor(*anchor, *state);
    }

    let box_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let decor = Label::new(None);
    decor.set_css_classes(&["purple"]);
    box_container.append(&decor);

    let hosts: Vec<Host> = hostnames
        .iter()
        .map(|hostname| {
            let hostname_label = Label::new(Some(&hostname.to_uppercase()));
            hostname_label.set_single_line_mode(true);
            hostname_label.set_css_classes(&["hostname-loading"]);

            box_container.append(&hostname_label);
            Host {
                hostname: hostname.to_string(),
                label: hostname_label,
            }
        })
        .collect();

    for host in hosts.clone() {
        glib::spawn_future_local(async move {
            let mut attempts = 0;
            while poll_server(&host.hostname).is_err() && attempts < 6 {
                glib::timeout_future_seconds(5).await;
                attempts += 1;
            }
            update_label(&host.label, &host.hostname);
        });
    }

    let tick = move || {
        for host in &hosts {
            update_label(&host.label, &host.hostname);
        }
        glib::ControlFlow::Continue
    };

    glib::timeout_add_seconds_local(60 * 15, tick);

    window.set_child(Some(&box_container));
    window.present();
}

fn main() -> ExitCode {
    let application = Application::builder()
        .application_id("com.cch000.rbar")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    application.connect_startup(|_| load_css());

    application.connect_command_line(|app, cmdline| {
        let args = cmdline.arguments();
        let hostnames: Vec<String> = args
            .iter()
            .skip(1)
            .map(|s| s.clone().into_string().unwrap())
            .collect();

        if hostnames.is_empty() {
            eprintln!("No arguments provided");
            return ExitCode::FAILURE;
        }

        activate_with_hostnames(app, &hostnames);

        ExitCode::SUCCESS
    });

    application.run()
}
