use gio::glib::{self};
use gtk4::gdk::Display;
use gtk4::{Application, ApplicationWindow, Label};
use gtk4::{CssProvider, prelude::*};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize)]
struct Info {
    status: String,
    availmem: f64,
    totalmem: f64,
    usedmem: f64,
    loadvg: Vec<f64>,
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_path("style.css");

    // Add the provider to the default screen
    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn poll_server(hostname: &str) -> Result<Info, Box<dyn Error>> {
    let Ok(res) = reqwest::blocking::get(format!("http://{hostname}:8114/status")) else {
        return Err("Failed to connect to server".into());
    };

    Ok(res.json::<Info>()?)
}

fn activate_with_hostnames(application: &Application, hostnames: Vec<String>) {
    let window = ApplicationWindow::new(application);

    window.init_layer_shell();
    window.set_layer(Layer::Background);
    window.set_default_height(28);

    let anchors = [
        (Edge::Left, true),
        (Edge::Right, true),
        (Edge::Top, true),
        (Edge::Bottom, false),
    ];

    for (anchor, state) in &anchors {
        window.set_anchor(*anchor, *state);
    }

    let box_container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

    for hostname in hostnames {
        let hostname = hostname.to_string();

        let hostname_label = Label::new(Some(&hostname));
        hostname_label.set_single_line_mode(true);
        hostname_label.set_text(&hostname);

        let status_label = Label::new(None);
        status_label.set_single_line_mode(true);
        status_label.set_text("");

        let host_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        host_box.append(&hostname_label);
        host_box.append(&status_label);

        match poll_server(&hostname) {
            Ok(info) => {
                if info.status == "running" {
                    status_label.set_markup("<span foreground=\"green\"></span>");
                } else {
                    status_label.set_markup("<span foreground=\"red\"></span>");
                }
            }
            Err(_) => status_label.set_markup("<span foreground=\"purple\"></span>"),
        }

        box_container.append(&host_box);

        let tick = move || {
            //label.set_text(&build_text(&hostname));
            glib::ControlFlow::Continue
        };

        glib::timeout_add_seconds_local(60 * 15, tick);
    }


    window.set_child(Some(&box_container));
    window.show();
}

fn main() {
    let application = Application::builder()
        .application_id("com.example.HephaestusStatus")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    application.connect_startup(|_| load_css());

    // Handle command line arguments properly using connect_command_line
    application.connect_command_line(|app, cmdline| {
        let args = cmdline.arguments();
        let hostnames: Vec<String> = args
            .iter()
            .skip(1) // Skip the program name
            .map(|s| s.clone().into_string().unwrap())
            .collect();

        activate_with_hostnames(app, hostnames);
        0.into()
    });

    application.run();
}
