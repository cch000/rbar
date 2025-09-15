use gio::glib::{self, ExitCode};
use gtk4::gdk::Display;
use gtk4::subclass::window;
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
    let Ok(mut res) = ureq::get(format!("http://{hostname}:8114/status")).call() else {
        return Err(format!("Failed to connect to server {hostname}").into());
    };

    Ok(res.body_mut().read_json::<Info>()?)
}

fn update_label(label: &Label, hostname: &str) {
    let css_classes: &[&str];
    let text: &str;

    if let Ok(info) = poll_server(hostname) {
        if info.status == "running" {
            text = "";
            css_classes = &["hostname-running"];
        } else {
            text = hostname;
            css_classes = &["hostname-not-running"];
        }
    } else {
        text = hostname;
        css_classes = &["hostname-unreachable"];
    }

    label.set_text(&text.to_uppercase());
    label.set_css_classes(css_classes);
}

fn activate_with_hostnames(application: &Application, hostnames: &[String]) {
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

    let box_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

    let hosts: Vec<Host> = hostnames
        .iter()
        .map(|hostname| {
            let hostname_label = Label::new(None);
            hostname_label.set_single_line_mode(true);
            hostname_label.set_css_classes(&["hostname-loading"]);

            box_container.append(&hostname_label);
            Host {
                hostname: hostname.to_string(),
                label: hostname_label,
            }
        })
        .collect();

    window.set_child(Some(&box_container));
    window.present();

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

        //reset window size to the minimum
        window.set_default_size(-1, -1);

        glib::ControlFlow::Continue
    };

    glib::timeout_add_seconds_local(60, tick);
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
