use async_std::task::sleep;
use chrono::Local;
use gtk::gdk;
use gtk::prelude::*;
use gtk::{
    gio,
    glib::{self, idle_add_local, spawn_future_local, timeout_add_local, ControlFlow, MainContext},
    Application, ApplicationWindow, Box, Button, CenterBox, CssProvider, EventControllerMotion,
    GestureClick, Label, Orientation, Overlay, Revealer, RevealerTransitionType, Widget,
};
use gtk4 as gtk;
use serde::Deserialize;
use serde_json::from_str;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env::var;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;
use tokio::sync::broadcast as tokio_broadcast;
use tokio::sync::mpsc as tokio_mpsc;
use async_std::task::spawn;
use zbus::connection::Builder;

async fn new() -> Box {
    let bus = Builder::session()
        .unwrap()
        .internal_executor(false)
        .build()
        .await
        .unwrap();
    spawn(async move {
        bus.executor().tick().await;
    });
    Box::default()
}

//NOTE :: wlr-tray is StatusNotifierItem stuff uses D-bus need to implement StatusNotifierHost
//        and display with gtk4 use xdg-popup to open popup-menu that's pretty much it
