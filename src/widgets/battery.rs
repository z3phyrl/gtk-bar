use chrono::Local;
use gdk::Display;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{self, timeout_add_local, ControlFlow},
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    GestureClick, Label, Orientation, Overlay, Revealer, RevealerTransitionType, Widget,
};
use gtk4 as gtk;
use gtk4_layer_shell as layer_shell;
use layer_shell::{Edge, Layer, LayerShell};
use sass_rs::{compile_string, Options};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

fn fmt() -> String {
    let percent = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .unwrap_or(" ".to_string());
    let status =
        std::fs::read_to_string("/sys/class/power_supply/BAT0/status").unwrap_or("󰂃 ".to_string());
    let icons = ["󰂎", "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰁹", "󰂃"];
    format!(
        "{}{} {}",
        icons[if let Ok(percent) = percent.trim().parse::<usize>() {
            percent / 10
        } else {
            11
        }],
        if status.trim() == "Charging" {
            "󱐋"
        } else {
            " "
        },
        percent.trim(),
    )
}

pub fn new() -> Box {
    let widget = Box::default();
    let batt = Label::new(Some(&fmt()));
    batt.add_css_class("battery");
    widget.append(&batt);
    timeout_add_local(Duration::from_secs(1), move || {
        batt.set_label(&fmt());
        ControlFlow::Continue
    });
    widget
}
