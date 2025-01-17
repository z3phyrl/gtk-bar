use async_std::prelude::*;
use async_std::task::sleep;
use chrono::Local;
use gdk::Display;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{
        self, clone, idle_add, idle_add_local, signal::Propagation, spawn_future,
        spawn_future_local, timeout_add_local, ControlFlow,
    },
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    EventControllerScroll, EventControllerScrollFlags, GestureClick, Label, Orientation::{Vertical, Horizontal}, Overlay,
    Revealer, RevealerTransitionType::{Crossfade, SlideLeft, SlideRight}, Scale, Widget,
};
use gtk4 as gtk;
use gtk4_layer_shell as layer_shell;
use layer_shell::{Edge, Layer, LayerShell};
use sass_rs::{compile_string, Options};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use std::{collections::HashMap, thread};
use tokio::sync::mpsc;

use serde::Deserialize;
use serde_json::from_str;
mod libs;
mod widgets;
mod windows;
use libs::hyprland;
use libs::shared_widget::spacer;
use widgets::{
    battery, clock,
    root::{self, Root},
    systray, volume, workspaces, music,
};

fn build_ui(app: &Application) {
    let mut hyprland = hyprland::new();
    let mut root = root::new(hyprland.listener());
    root.left.set_spacing(5);
    root.center.set_spacing(15);
    root.right.set_spacing(15);
    root.left(&spacer(15));
    root.left(&workspaces::new(hyprland.listener()));
    root.left(&music::new());

    // root.center();

    root.right(&volume::new(app));
    root.right(&systray::new(root.listen()));
    root.right(&clock::new());
    if let Some(batt) = battery::new() {
        root.right(&batt.widget);
    }
    root.right(&spacer(0));

    window(app, &root);
    async_std::task::spawn(async move {
        hyprland.listen().await;
    });
}

fn window(app: &Application, root: &Root) {
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
