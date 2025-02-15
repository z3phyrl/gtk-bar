use crate::*;
use anyhow::Result;
use gtk4::Align;
use libs::pulse::{change_channel_volume_by_percent, channel_volume_by_percent, Pulse};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
pub struct PopUpVolume {
    window: ApplicationWindow,
    hover: EventControllerMotion,
    timingout: Rc<Cell<bool>>,
    value: Label,
    scale: Scale,
}

impl PopUpVolume {
    pub fn new(app: &Application) -> Self {
        let pulse_info = Pulse::new("z3phyrl.popup-volume.info").unwrap();
        let pulse_ctl = Pulse::new("z3phyrl.popup-volume.ctl").unwrap();
        let widget = Overlay::new();
        widget.add_css_class("popup-volume");
        let scale = Scale::with_range(Horizontal, 0.0, 100.0, 1.0);
        let value = Label::new(None);
        scale.connect_change_value(move |s, t, v| {
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
        });

        value.set_halign(Align::End);
        value.set_margin_end(20);
        scale.set_inverted(true);
        widget.add_overlay(&scale);
        widget.add_overlay(&value);

        let window = ApplicationWindow::builder()
            .application(app)
            .css_classes(["popup-volume-window"])
            .default_width(256 + 20) // +20 is for the 10px margin on each side
            .default_height(50 + 20)
            .child(&widget)
            .build();
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Right, true);

        let hover = EventControllerMotion::new();
        window.add_controller(hover.clone());
        let timingout = Rc::new(Cell::new(false));
        let this = Self {
            window,
            hover: hover.clone(),
            timingout,
            value,
            scale,
        };
        hover.connect_contains_pointer_notify(clone! {
            #[strong] this,
            move |h| {
                if !h.contains_pointer() {
                    this.timeout();
                }
            }
        });
        this
    }
    fn timeout(&self) {
        spawn_future_local(clone! {
            #[strong(rename_to = window)] self.window,
            #[strong(rename_to = hover)] self.hover,
            #[strong(rename_to = timingout)]  self.timingout,
            async move {
                let mut timeout = 200;
                timingout.set(false);
                sleep(Duration::from_millis(20)).await;
                timingout.set(true);
                while timingout.get() && timeout > 0 && !hover.contains_pointer() {
                    println!("{timeout}");
                    sleep(Duration::from_millis(10)).await;
                    timeout -= 1;
                }
                if timeout == 0 {
                    window.hide(); // for some reason destroy doesn't work
                }
            }
        });
    }
    pub fn update(&self, text: &str, value: f64) {
        self.value.set_text(text);
        self.scale.set_value(value);
    }
    pub fn present(&self, present: bool) {
        if present {
            self.window.present();
            self.timeout();
        }
    }
    pub fn presenting(&self) -> bool {
        self.window.is_mapped()
    }
}
