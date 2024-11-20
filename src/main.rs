use chrono::Local;
use gdk::Display;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{self, idle_add, idle_add_local, timeout_add_local,spawn_future, ControlFlow},
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    GestureClick, Label, Orientation, Overlay, Revealer, RevealerTransitionType, Widget,
};
use gtk4 as gtk;
use gtk4_layer_shell as layer_shell;
use layer_shell::{Edge, Layer, LayerShell};
use sass_rs::{compile_string, Options};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

mod libs;
mod widgets;
use libs::hyprland;
use widgets::battery;
use widgets::clock;
use widgets::root;
use widgets::workspaces::HyprlandWorkspacesExt;

fn build_ui(app: &Application) {
    let mut hyprland = hyprland::new();
    let reveal = Button::builder().label("Hello, World!").build();
    let spacer = Box::default();
    let mut root = root::new();
    root.spacing(20);
    let workspace = hyprland.workspaces();
    root.left(vec![&spacer, &workspace]);
    root.center(vec![&reveal]);
    root.right(vec![&clock::new(), &battery::new(), &spacer]);

    window(app, &root);
    async_std::task::spawn(async move {
        hyprland.listen().await;
    });

    reveal.connect_clicked(move |_| {
        root.clone().transparent(!root.transparency());
    });
}

fn window(app: &Application, root: &root::Root) {
    let window = ApplicationWindow::builder()
        .application(app)
        .css_classes(["bar"])
        .default_width(1920)
        .default_height(50)
        .child(&root.widget())
        .build();

    window.init_layer_shell();
    window.auto_exclusive_zone_enable();
    window.set_layer(Layer::Top);
    window.set_anchor(Edge::Bottom, true);

    window.present();
}

fn load_css() {
    let provider = CssProvider::new();
    let css = compile_string(include_str!("style.scss"), Options::default())
        .expect("Error compileing scss");
    provider.load_from_string(&css);

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
}

#[tokio::main]
async fn main() -> glib::ExitCode {
    let app = Application::builder().application_id("bar").build();

    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);

    app.run()
}
