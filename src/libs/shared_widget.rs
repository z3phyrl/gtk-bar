use crate::*;

#[derive(Clone)]
pub struct CrossfadeIn {
    pub widget: Revealer,
    slidein: Revealer,
    crossfade: Revealer,
    duration: Duration,
}

impl CrossfadeIn {
    pub fn new<W>(child: &W, duration: Duration) -> Self
    where
        W: IsA<Widget>,
    {
        let crossfade = Revealer::builder()
            .transition_type(Crossfade)
            .transition_duration(duration.as_millis() as u32)
            .child(child)
            .build();
        let slidein = Revealer::builder()
            .transition_type(SlideRight)
            .transition_duration(duration.as_millis() as u32 / 2)
            .child(&crossfade)
            .build();
        Self {
            widget: slidein.clone(),
            slidein,
            crossfade,
            duration,
        }
    }
    pub fn reveal(&self, reveal: bool) {
        if reveal {
            spawn_future_local(clone! {
                #[strong(rename_to = this)] self,
                async move {
                    this.slidein.set_reveal_child(reveal);
                    sleep(this.duration / 6).await;
                    this.crossfade.set_reveal_child(reveal);
                }
            });
        } else {
            spawn_future_local(clone! {
                #[strong(rename_to = this)] self,
                async move {
                    this.crossfade.set_reveal_child(reveal);
                    sleep(this.duration / 2).await;
                    this.slidein.set_reveal_child(reveal);
                }
            });
        }
    }
    pub fn revealed(&self) -> bool {
        self.slidein.reveals_child() || self.crossfade.reveals_child()
    }
}

pub fn spacer(space: i32) -> Box {
    Box::builder().margin_start(space / 2).margin_end(space / 2).build()
}
