use core::fmt;

use gio::glib;
use gtk4::{
    Application, Label,
    prelude::{BoxExt, GtkWindowExt, WidgetExt},
};
use ureq::Agent;

use crate::tools::{layer_setup, poll_server, ureq_setup};

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
        self.0.fmt(f)
    }
}

const STATUS_ICON: &str = "";
const CONNECTION_ICON: &str = "󰊱";

#[derive(Clone)]
struct HostBox {
    connection: Label,
    status: Label,
    hostname: Hostname,
    ip: Ip,
}

impl HostBox {
    fn update(&self, agent: &Agent) {
        let hostname = &self.hostname;
        let ip = &self.ip;
        let connection = &self.connection;
        let status = &self.status;

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

    fn setup(box_container: &gtk4::Box, host_input: &str) -> Self {
        let (hostname, ip) = host_input.split_once(':').unwrap();
        let hostname = hostname.to_uppercase();

        let host_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        host_box.set_css_classes(&["host_box"]);

        let status = Label::new(Some(STATUS_ICON));
        status.set_css_classes(&["grey"]);

        let connection = Label::new(Some(CONNECTION_ICON));
        connection.set_css_classes(&["grey"]);

        let hostname_label = Label::new(Some(&hostname));
        hostname_label.set_css_classes(&["hostname"]);
        hostname_label.set_xalign(1.0);
        hostname_label.set_hexpand(true);

        host_box.append(&hostname_label);
        host_box.append(&status);
        host_box.append(&connection);

        box_container.append(&host_box);

        Self {
            ip: Ip(ip.to_owned()),
            hostname: Hostname(hostname),
            status,
            connection,
        }
    }
}

pub struct Ui;

impl Ui {
    pub fn activate(application: &Application, hostnames: &[String]) {
        let agent = ureq_setup();

        let window = layer_setup(application);

        let box_container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        let hosts: Vec<HostBox> = hostnames
            .iter()
            .map(|host_input| HostBox::setup(&box_container, host_input))
            .collect();

        window.set_child(Some(&box_container));
        window.present();

        Self::populate(&agent, &hosts);

        let tick = move || {
            for host in &hosts {
                HostBox::update(host, &agent);
            }

            glib::ControlFlow::Continue
        };

        glib::timeout_add_seconds_local(60, tick);
    }

    fn populate(agent: &Agent, hosts: &[HostBox]) {
        for host in hosts.iter().cloned() {
            let agent = agent.clone();
            glib::spawn_future_local(async move {
                let mut attempts = 0;
                while poll_server(&agent, &host.hostname.to_string()).is_err() && attempts < 6 {
                    glib::timeout_future_seconds(5).await;
                    attempts += 1;
                }

                HostBox::update(&host, &agent);
            });
        }
    }
}
