use gio::{
    glib::ExitCode,
    prelude::{ApplicationCommandLineExt, ApplicationExt, ApplicationExtManual},
};
use gtk4::{Application, CssProvider, gdk::Display};

mod tools;
mod ui;
use crate::ui::Ui;

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../style.css"));

    gtk4::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
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

        Ui::activate(app, &hostnames);

        ExitCode::SUCCESS
    });

    application.run()
}
