use crate::*;
use anyhow::{Error, Result};
use async_std::channel::{unbounded, Receiver};
use pulseaudio::protocol::{
    self, command::SinkInfo, ChannelVolume, SetDeviceMuteParams, SetDeviceVolumeParams,
    SubscriptionEvent, SubscriptionEventFacility, SubscriptionEventType, Volume,
};
use std::{
    cell::{Cell, RefCell},
    ffi::CString,
    io::{BufRead, BufReader},
    os::unix::net::UnixStream,
    path::PathBuf,
    rc::Rc,
};

#[derive(Clone)]
pub struct Pulse {
    sock: Rc<RefCell<UnixStream>>,
    version: u16,
}

impl Pulse {
    pub fn new(name: &str) -> Result<Self> {
        let socket_path = pulseaudio::socket_path_from_env().unwrap();
        let stream = UnixStream::connect(socket_path)?;
        let mut sock = BufReader::new(&stream);

        // PulseAudio usually puts an authentication "cookie" in ~/.config/pulse/cookie.
        let cookie = pulseaudio::cookie_path_from_env()
            .and_then(|path| std::fs::read(path).ok())
            .unwrap_or_default();
        let auth = protocol::AuthParams {
            version: protocol::MAX_VERSION,
            supports_shm: false,
            supports_memfd: false,
            cookie: cookie.clone(),
        };

        // Write the auth "command" to the socket, and read the reply. The reply
        // contains the negotiated protocol version.
        protocol::write_command_message(
            sock.get_mut(),
            0,
            protocol::Command::Auth(auth),
            protocol::MAX_VERSION,
        )?;

        let (_, auth_reply) =
            protocol::read_reply_message::<protocol::AuthReply>(&mut sock, protocol::MAX_VERSION)?;
        let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_reply.version);

        // The next step is to set the client name.
        let mut props = protocol::Props::new();
        props.set(protocol::Prop::ApplicationName, CString::new(name).unwrap());
        protocol::write_command_message(
            sock.get_mut(),
            1,
            protocol::Command::SetClientName(props),
            protocol_version,
        )?;

        // The reply contains our client ID.
        let (_seq, _id) = protocol::read_reply_message::<protocol::SetClientNameReply>(
            &mut sock,
            protocol_version,
        )?;

        Ok(Self {
            sock: Rc::new(RefCell::new(stream)),
            version: protocol_version,
        })
    }
    pub fn subscribe(&self) -> Result<Receiver<SubscriptionEvent>> {
        // Finally, write a command to create a subscription. The mask we pass will
        // determine which events we get.
        // let mut sock = self.sock.into_inner();
        let mut sock = BufReader::new(self.sock.borrow().try_clone()?);
        protocol::write_command_message(
            &mut sock.get_mut(),
            2,
            protocol::Command::Subscribe(protocol::SubscriptionMask::ALL),
            self.version,
        )?;

        // The first reply is just an ACK.
        let seq = protocol::read_ack_message(&mut sock)?;
        assert_eq!(2, seq);

        let (tx, rx) = unbounded();
        // let mut sock = BufReader::new(self.sock.get_mut().try_clone()?);

        tokio::task::spawn_blocking(clone! {
            #[strong(rename_to = version)] self.version,
            move || loop {
            if let Ok((_, event)) = protocol::read_command_message(&mut sock, version) {
                match event {
                    protocol::Command::SubscribeEvent(event) => {
                        tx.send_blocking(event).unwrap();
                    }
                    _ => eprintln!("got unexpected event {:?}", event),
                }
            }
        }});
        Ok(rx)
    }
    /// use index 0 to get default sink info
    pub fn get_sink_info(&self, index: u32) -> Result<SinkInfo> {
        let mut sock = BufReader::new(self.sock.borrow().try_clone()?);
        protocol::write_command_message(
            sock.get_mut(),
            0,
            protocol::Command::GetSinkInfo(protocol::command::GetSinkInfo {
                index: Some(index),
                name: None,
            }),
            self.version,
        )?;
        let (seq, reply) =
            protocol::read_reply_message::<protocol::SinkInfo>(&mut sock, self.version)?;
        Ok(reply)
    }
    pub fn set_sink_mute(&self, index: u32, mute: bool) -> Result<()> {
        let mut sock = self.sock.borrow().try_clone()?;
        Ok(protocol::write_command_message(
            &mut sock,
            0,
            protocol::Command::SetSinkMute(SetDeviceMuteParams {
                device_index: Some(index),
                device_name: None,
                mute,
            }),
            self.version,
        )?)
    }
    pub fn set_sink_volume(&self, index: u32, volume: ChannelVolume) -> Result<()> {
        let mut sock = self.sock.borrow().try_clone()?;
        Ok(protocol::write_command_message(
            &mut sock,
            0,
            protocol::Command::SetSinkVolume(SetDeviceVolumeParams {
                device_index: Some(index),
                device_name: None,
                volume,
            }),
            self.version,
        )?)
    }
}

pub fn channel_volume_by_percent(base_volume: Volume, channel: u32, percent: f64) -> ChannelVolume {
    let mut channel_volume = ChannelVolume::empty();
    let volume = Volume::from_u32_clamped((base_volume.as_u32() as f64 * percent) as u32);
    for _i in 0..channel {
        channel_volume.push(volume);
    }
    channel_volume
}
pub fn change_channel_volume_by_percent(
    base_volume: Volume,
    channel_volume: ChannelVolume,
    percent: f64,
) -> ChannelVolume {
    let base_vol = base_volume.as_u32() as f64;
    let channels = channel_volume.channels();
    let mut result = ChannelVolume::empty();
    for volume in channels {
        result.push(Volume::from_u32_clamped(
            (volume.as_u32() as i32 + (base_vol * percent) as i32) as u32,
        ));
    }
    result
}
