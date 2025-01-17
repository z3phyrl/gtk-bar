use crate::*;
use async_broadcast::Receiver;
use hyprland::ctl;
use std::process::Command;

pub fn new(mut listen_to: Receiver<bool>) -> Box {
    let widget = Box::default();
    widget.add_css_class("container");
    widget.add_css_class("systray");
    widget.append(&Label::new(Some("^")));
    let lclick = GestureClick::new();
    Command::new("eww")
        .args(["open", "temp-gtk-bar-systray"])
        .spawn()
        .unwrap();
    lclick.connect_pressed(move |click, count, x, y| {
        spawn_future(async move {
            if let Ok(res) = Command::new("eww").args(["get", "temp-systray"]).output() {
                let opened = if res.stdout == "true\n".as_bytes() {
                    true
                } else {
                    false
                };
                println!("{opened:?}");
                let _ = Command::new("eww")
                    .args(["update", &format!("temp-systray-reveal={}", !opened)])
                    .spawn();
                if opened {
                    sleep(Duration::from_millis(100)).await;
                } else {
                    for i in 272..274 + 1 {
                        ctl(&format!(r#"keyword bindn ,mouse:{i},exec,bash -c "[[ $(eww get temp-systray-focus) == "false" ]] && eww update temp-systray-reveal=false && hyprctl --batch 'keyword unbind ,mouse:272;keyword unbind ,mouse:273;keyword unbind ,mouse:274;' && sleep 0.1 && eww update temp-systray=false" "#));
                    }
                }
                let _ = Command::new("eww")
                    .args(["update", &format!("temp-systray={}", !opened)])
                    .spawn();
            }
        });
    });
    widget.add_controller(lclick);
    spawn_future_local(async move {
        loop {
            if let Ok(opaque) = listen_to.recv_direct().await {
                spawn_future_local(async move {
                    let _ = Command::new("eww")
                        .args(["update", &format!("temp-systray-opaque={}", opaque)])
                        .spawn();
                });
            }
        }
    });
    widget
}

//NOTE :: wlr-tray is StatusNotifierItem stuff uses D-bus need to implement StatusNotifierHost
//        and display with gtk4 use xdg-popup to open popup-menu that's pretty much it
