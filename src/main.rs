use gio::glib::{self, ExitCode, clone};
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

    let labels: Vec<Label> = hostnames
        .iter()
        .map(|hostname| {
            let label = Label::new(Some(hostname));
            label.set_single_line_mode(true);
            label.set_css_classes(&["hostname-loading"]);

            box_container.append(&label);
            label
        })
        .collect();

    let hostnames_clone = hostnames.clone();

    //Give some margin during startup
    labels
        .iter()
        .zip(hostnames_clone)
        .for_each(|(label, hostname)| {
            glib::spawn_future_local(clone!(
                #[weak]
                label,
                async move {
                    let mut counter = 0;
                    while poll_server(&hostname).is_err() && counter < 6 {
                        glib::timeout_future_seconds(5).await;
                        counter += 1;
                    }
                    update_label(&label, &hostname);
                }
            ));
        });

    let tick = move || {
        labels.iter().zip(&hostnames).for_each(|(label, hostname)| {
            update_label(label, hostname);
        });
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

        activate_with_hostnames(app, hostnames);
        0.into()
    });

    application.run();

    ExitCode::SUCCESS
}
