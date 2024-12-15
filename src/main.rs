use async_std::prelude::*;
use async_std::task::sleep;
use chrono::Local;
use gdk::Display;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{
        self, clone, idle_add, idle_add_local, spawn_future, spawn_future_local, timeout_add_local,
        ControlFlow,
    },
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    GestureClick, Label, Orientation, Overlay, Revealer, RevealerTransitionType, Widget,
};
use gtk4 as gtk;
use gtk4_layer_shell as layer_shell;
use layer_shell::{Edge, Layer, LayerShell};
use sass_rs::{compile_string, Options};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{collections::HashMap, thread};
use tokio::sync::mpsc;

use serde::Deserialize;
use serde_json::from_str;

mod libs;
mod widgets;
use libs::hyprland::{self, Controller, Hyprland};
use widgets::{
    battery, clock,
    root::{HyprlandRootExt, Root},
    systray,
    workspaces::HyprlandWorkspacesExt,
};

fn build_ui(app: &Application) {
    let mut hyprland = hyprland::new();
    let hyprctl = hyprland.controller();
    let spacer = || -> Box { Box::default() };
    let mut root = hyprland.root();
    root.spacing(20);
    let workspace = hyprland.workspaces();
    let music = Box::default();
    music.append(&Label::new(Some("Music")));
    root.left(&spacer());
    root.left(&workspace);
    root.left(&music);

    // root.center();

    root.right(&systray::new(root.listen(), hyprctl));
    root.right(&clock::new());
    if let Some(batt) = battery::new() {
        root.right(&batt.widget);
    }
    root.right(&spacer());

    window(app, &root);
    async_std::task::spawn(async move {
        hyprland.listen().await;
    });
}

fn window(app: &Application, root: &Root) {
    let window = ApplicationWindow::builder()
        .application(app)
        .css_classes(["bar"])
        .default_width(2560)
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
