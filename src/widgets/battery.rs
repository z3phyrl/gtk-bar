use crate::*;
use std::path::Path;

pub struct Battery {
    pub widget: Box,
}

impl Battery {
    pub fn has() -> bool {
        Path::new("/sys/class/power_supply/BAT0").exists()
            && Path::new("/sys/class/power_supply/BAT0/status").exists()
            && Path::new("/sys/class/power_supply/BAT0/capacity").exists()
    }
    pub fn percent() -> Option<usize> {
        if let Ok(percent) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
            percent.trim().parse::<usize>().ok()
        } else {
            None
        }
    }
    pub fn status() -> Option<String> {
        std::fs::read_to_string("/sys/class/power_supply/BAT0/status").ok()
    }
    pub fn fmt() -> String {
        if let Some(percent) = Self::percent() {
            let status = Self::status().unwrap_or("󰂃 ".to_string());
            let icons = ["󰂎", "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰁹", "󰂃"];
            format!(
                "{}{} {}",
                icons[percent / 10],
                if status.trim() == "Charging" {
                    "󱐋"
                } else {
                    " "
                },
                percent,
            )
        } else {
            String::new()
        }
    }
    pub fn new() -> Option<Self> {
        if Self::has() {
            let widget = Box::default();
            let batt = Label::new(Some(&Self::fmt()));
            batt.add_css_class("battery");
            widget.append(&batt);
            timeout_add_local(Duration::from_secs(1), move || {
                batt.set_label(&Self::fmt());
                ControlFlow::Continue
            });
            Some(Self { widget })
        } else {
            None
        }
    }
}

pub fn new() -> Option<Battery> {
    Battery::new()
}
