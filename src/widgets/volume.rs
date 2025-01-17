use crate::*;
use anyhow::Result;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use libs::pulse::{change_channel_volume_by_percent, channel_volume_by_percent, Pulse};
use pulseaudio::protocol::command::{
    SinkInfo, SubscriptionEvent, SubscriptionEventFacility, SubscriptionEventType,
};
use windows::popup_volume::PopUpVolume;

pub fn get_volume(info: &SinkInfo) -> Result<f32> {
    let base_vol = info.base_volume.as_u32() as f32;
    let channels = info.cvolume.channels();
    let mut avg = 0.0;
    for volume in channels {
        avg += volume.as_u32() as f32;
    }
    Ok(((avg / channels.len() as f32 * 100.0) / base_vol).floor())
}

pub fn get_icon(mute: bool, volume: f32) -> String {
    String::from(if mute {
        "󰝟"
    } else if volume > 66.0 {
        "󰕾"
    } else if volume > 33.0 {
        "󰖀"
    } else {
        "󰕿"
    })
}

pub fn new(app: &Application) -> Box {
    // TODO :: maybe reducing connection to pulseaudio if nessesary
    let pulse_event = libs::pulse::Pulse::new("z3phyrl.gtk-bar.event").unwrap();
    let pulse_ctl = libs::pulse::Pulse::new("z3phyrl.gtk-bar.ctl").unwrap();
    let pulse_info = libs::pulse::Pulse::new("z3phyrl.gtk-bar.info").unwrap();
    let subscription = pulse_event.subscribe().unwrap();
    let popup_volume = PopUpVolume::new(app);

    let widget = Box::new(Horizontal, 10);
    widget.add_css_class("container");
    widget.add_css_class("volume");
    let icon = Label::new(None);
    let scale = Scale::with_range(Horizontal, 0.0, 100.0, 1.0);
    let revealer = Revealer::builder()
        .transition_type(SlideRight)
        .child(&scale)
        .build();
    let label = Label::new(None);
    widget.append(&icon);
    widget.append(&revealer);
    widget.append(&label);

    // set up mouse actions
    let mute = GestureClick::new();
    let expand = GestureClick::new();
    let motion = EventControllerMotion::new();
    let inhibit = EventControllerMotion::new();
    let hover = Rc::new(Cell::new(false));
    let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
    expand.connect_pressed(clone! {
        #[strong] revealer,
        #[strong] hover,
        move |_, _, _, _| {
            if hover.get() {
                revealer.set_reveal_child(true);
            }
        }
    });
    let timeout_hover = clone! {
    #[strong] revealer,
    #[strong] hover,
    move |m: &EventControllerMotion| {
        if !m.contains_pointer() { return };
        spawn_future_local(clone! {
            #[strong] m,
            #[strong] revealer,
            #[strong] hover,
            async move {
                // println!("notify");
                if !hover.get() {
                    hover.set(true);
                    let mut timeout = 200;
                    while timeout > 0 && hover.get() && m.contains_pointer() {
                        sleep(Duration::from_millis(10)).await;
                        timeout -= 1;
                    }
                    if timeout == 0 {
                        revealer.set_reveal_child(true);
                    } else if hover.get() {
                        hover.set(false);
                    }
                }
            }
        });
    }};
    motion.connect_contains_pointer_notify(clone! {
        #[strong] timeout_hover,
        #[strong] revealer,
        #[strong] hover,
        move |m| {
            if m.contains_pointer() {
                timeout_hover(m);
            } else {
                hover.set(false);
                revealer.set_reveal_child(false);
            }
        }
    });
    inhibit.connect_contains_pointer_notify(clone! {
        #[strong] hover,
        #[strong] motion,
        #[strong] timeout_hover,
        move |i| {
            if i.contains_pointer() {
                hover.set(false);
            } else {
                timeout_hover(&motion);
            }
        }
    });
    mute.connect_pressed(clone! {
        #[strong] pulse_info,
        #[strong] pulse_ctl,
        move |_, _, _, _| {
            if let Ok(info) = pulse_info.get_sink_info(0) {
                pulse_ctl.set_sink_mute(0, !info.muted).unwrap();
            }
        }
    });
    // TODO :: could get info once when new or remove sink event fire instead of getting it every time
    let sensitivity = 0.5;
    scroll.connect_scroll(clone! {
        #[strong] pulse_info,
        #[strong] pulse_ctl,
        move |s, dx, dy| -> Propagation {
            if dy < -sensitivity {
                // print!("Ok ");
                if let Ok(info) = pulse_info.get_sink_info(0) {
                    pulse_ctl.set_sink_volume(0, change_channel_volume_by_percent(info.base_volume, info.cvolume, 0.01)).unwrap();
                    // print!(":: Up");
                }
                // println!("");
            } else if dy > sensitivity {
                // print!("Ok ");
                if let Ok(info) = pulse_info.get_sink_info(0) {
                    pulse_ctl.set_sink_volume(0, change_channel_volume_by_percent(info.base_volume, info.cvolume, -0.01)).unwrap();
                    // print!(":: Down");
                }
                // println!("");
            }
            Propagation::Proceed
        }
    });
    scale.connect_change_value(clone! {
        #[strong] pulse_info,
        #[strong] pulse_ctl,
        move |s, t, v| -> Propagation {
            // println!("{t:#?}");
            if let Ok(info) = pulse_info.get_sink_info(0) {
                pulse_ctl
                    .set_sink_volume(
                        0,
                        channel_volume_by_percent(
                            info.base_volume,
                            info.cvolume.channels().len() as u32,
                            v * 0.01,
                        ),
                    )
                    .unwrap();
            }
            Propagation::Proceed
        }
    });
    widget.add_controller(expand);
    widget.add_controller(motion);
    widget.add_controller(scroll);
    icon.add_controller(inhibit);
    icon.add_controller(mute);

    // set up event subcription

    spawn_future_local(async move {
        let update = move || {
            if let Ok(info) = pulse_info.get_sink_info(0) {
                if let Ok(volume) = get_volume(&info) {
                    icon.set_text(&get_icon(info.muted, volume));
                    // TODO :: maybe make the scale interpolate between values
                    scale.set_value(volume as f64);
                    label.set_text(&format!("{volume:.0}%"));
                    // if !hover.get() {
                    //     popup_volume.present(true);
                    // }
                    // if popup_volume.presenting() {
                    //     popup_volume.update(
                    //         &format!("{} {volume:.0}%", &get_icon(info.muted, volume)),
                    //         volume as f64,
                    //     );
                    // }
                }
            }
        };
        update();
        loop {
            match subscription.recv().await {
                Ok(SubscriptionEvent {
                    event_facility: SubscriptionEventFacility::Sink,
                    event_type: SubscriptionEventType::Changed,
                    index: _,
                }) => {
                    update();
                    // println!("SINK => {index:?}");
                }
                Ok(SubscriptionEvent {
                    event_facility: SubscriptionEventFacility::Sink,
                    event_type: SubscriptionEventType::New,
                    index: _,
                })
                | Ok(SubscriptionEvent {
                    event_facility: SubscriptionEventFacility::Sink,
                    event_type: SubscriptionEventType::Removed,
                    index: _,
                }) => {
                    update();
                    // println!("CHANGED :: SINK => {index:?}");
                }
                Ok(_e) => {
                    // eprintln!("unexpected event {e:?}");
                }
                Err(_) => {}
            }
        }
    });
    widget
}

// TODO :: change to use pulseaudio subscribe events instead of alsa
// using libpulse_binding crate
