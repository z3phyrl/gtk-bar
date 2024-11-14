use chrono::Local;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{self, timeout_add_local, ControlFlow},
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    GestureClick, Label, Orientation, Overlay, Revealer, RevealerTransitionType, Widget,
};
use gtk4 as gtk;
use std::time::Duration;

pub fn new() -> Box {
    let widget = Box::new(Orientation::Horizontal, 5);
    let icon = Label::new(Some("󰥔 "));
    let time = Label::new(Some(&Local::now().format("%H : %M").to_string()));
    widget.append(&icon);
    widget.append(&time);
    timeout_add_local(Duration::from_secs(1), move || {
        let now = Local::now();
        let sec = format!("{}", now.format("%S"))
            .parse::<i32>()
            .expect("Datetime is broken some how");
        if sec % 2 == 0 {
            time.set_label(&format!("{}", now.format("%H : %M")));
        } else {
            time.set_label(&format!("{}", now.format("%H   %M")))
        }
        ControlFlow::Continue
    });
    widget
}
