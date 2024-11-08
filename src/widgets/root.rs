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
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Root {
    root: Overlay,
    bg: Revealer,
    left: Box,
    center: Box,
    right: Box,
}

impl Root {
    pub fn widget(&self) -> Overlay {
        self.root.clone()
    }
    pub fn left<W>(&mut self, widgets: Vec<&W>)
    where
        W: IsA<Widget>,
    {
        for widget in widgets {
            self.left.append(widget);
        }
    }
    pub fn center<W>(&mut self, widgets: Vec<&W>)
    where
        W: IsA<Widget>,
    {
        for widget in widgets {
            self.center.append(widget);
        }
    }
    pub fn right<W>(&mut self, widgets: Vec<&W>)
    where
        W: IsA<Widget>,
    {
        for widget in widgets {
            self.right.append(widget);
        }
    }
    pub fn transparent(&mut self, transparent: bool) {
        self.bg.set_reveal_child(transparent);
    }
    pub fn transparency(&self) -> bool {
        self.bg.reveals_child()
    }
    pub fn spacing(&mut self, spacing: i32) {
        self.left.set_spacing(spacing);
        self.center.set_spacing(spacing);
        self.right.set_spacing(spacing);
    }
}

pub fn new() -> Root {
    let bg = Revealer::builder()
        .transition_type(RevealerTransitionType::Crossfade)
        .transition_duration(500)
        .child(&Box::builder().css_classes(["bg"]).build())
        .reveal_child(true)
        .build();
    let root = Overlay::builder().child(&bg).build();
    let left = Box::new(Orientation::Horizontal, 0);
    let center = Box::new(Orientation::Horizontal, 0);
    let right = Box::new(Orientation::Horizontal, 0);
    let content = CenterBox::builder()
        .start_widget(&left)
        .center_widget(&center)
        .end_widget(&right)
        .build();
    root.add_overlay(&content);
    Root {
        root,
        bg,
        left,
        center,
        right,
    }
}
