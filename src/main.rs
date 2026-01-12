use gio::glib::{self, ExitCode};
use gtk4::{Application, ApplicationWindow, Label, prelude::*};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::fmt;
use ureq::Agent;

mod tools;
use crate::tools::{load_css, poll_server, ureq_setup};

#[derive(Clone, PartialEq, Eq)]
struct Hostname(String);

impl fmt::Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f) // Delegate to the inner String
    }
}

#[derive(Clone)]
struct Ip(String);

impl fmt::Display for Ip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f) // Delegate to the inner String
    }
}

#[derive(Clone)]
struct Host {
    connection: Label,
    status: Label,
    hostname: Hostname,
    ip: Ip,
}

const STATUS_ICON: &str = "";
const CONNECTION_ICON: &str = "󰊱";

fn update_host_box(agent: &Agent, host: &Host) {
    let hostname = &host.hostname;
    let ip = &host.ip;
    let connection = &host.connection;
    let status = &host.status;

    let (connection_color, info) = poll_server(agent, &hostname.to_string()).map_or_else(
        |_| {
            poll_server(agent, &ip.to_string())
                .map_or((&["red"], None), |info| (&["orange"], Some(info)))
        },
        |info| (&["green"], Some(info)),
    );

    let status_color = if let Some(info) = info {
        if info.status.eq("running") {
            &["green"]
        } else {
            &["orange"]
        }
    } else {
        &["red"]
    };
    connection.set_css_classes(connection_color);
    status.set_css_classes(status_color);
}

fn activate_with_hostnames(application: &Application, hostnames: &[String]) {
    let agent = ureq_setup();

    let window = layer_setup(application);

    let box_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let hosts: Vec<Host> = hostnames
        .iter()
        .map(|host_input| host_boxes_setup(&box_container, host_input))
        .collect();

    window.set_child(Some(&box_container));
    window.present();

    host_boxes_populate(&agent, &hosts);

    let tick = move || {
        for host in &hosts {
            update_host_box(&agent, host);
        }

        window.set_default_size(-1, -1);

        glib::ControlFlow::Continue
    };

    glib::timeout_add_seconds_local(60, tick);
}

fn layer_setup(application: &Application) -> ApplicationWindow {
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

fn host_boxes_populate(agent: &Agent, hosts: &[Host]) {
    for host in hosts.iter().cloned() {
        let agent = agent.clone();
        glib::spawn_future_local(async move {
            let mut attempts = 0;
            while poll_server(&agent, &host.hostname.to_string()).is_err() && attempts < 6 {
                glib::timeout_future_seconds(5).await;
                attempts += 1;
            }

            update_host_box(&agent, &host);
        });
    }
}

fn host_boxes_setup(box_container: &gtk4::Box, host_input: &str) -> Host {
    let (hostname, ip) = host_input.split_once(':').unwrap();
    let hostname = hostname.to_uppercase();
    let host_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let status = Label::new(Some(STATUS_ICON));
    status.set_css_classes(&["grey"]);

    let connection = Label::new(Some(CONNECTION_ICON));
    connection.set_css_classes(&["grey"]);

    let hostname_label = Label::new(Some(&hostname));
    hostname_label.set_css_classes(&["hostname"]);
    hostname_label.set_xalign(1.0);

    host_box.append(&hostname_label);
    host_box.append(&status);
    host_box.append(&connection);

    box_container.append(&host_box);

    Host {
        ip: Ip(ip.to_owned()),
        hostname: Hostname(hostname),
        status,
        connection,
    }
}

fn main() -> ExitCode {
    let application = Application::builder()
        .application_id("com.cch000.rbar")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    application.connect_startup(|_| load_css());

    application.connect_command_line(|app, cmdline| {
        let args = cmdline.arguments();
        let mut hostnames: Vec<String> = args
            .iter()
            .skip(1)
            .map(|s| s.clone().into_string().unwrap())
            .collect();

        hostnames.is_empty().then(|| {
            eprintln!("No arguments provided");
            ExitCode::FAILURE
        });

        for s in &hostnames {
            (!s.contains(':')).then(|| {
                eprintln!("Malformed input, should be hostname:ip");
                ExitCode::FAILURE
            });
        }

        hostnames.sort();

        activate_with_hostnames(app, &hostnames);

        ExitCode::SUCCESS
    });

    application.run()
}
