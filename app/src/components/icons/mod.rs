use leptos::*;

pub enum Icons {
    Info,
    Tick,
    Close,
    Mic,
    MicOff,
    Video,
    VideoOff,
}

impl Icons {
    pub fn svg(&self) -> &'static str {
        match self {
            Icons::Info => include_str!("info.svg"),
            Icons::Tick => include_str!("tick.svg"),
            Icons::Close => include_str!("close.svg"),
            Icons::Mic => include_str!("mic.svg"),
            Icons::MicOff => include_str!("mic_off.svg"),
            Icons::Video => include_str!("video.svg"),
            Icons::VideoOff => include_str!("video_off.svg"),
        }
    }
}

#[component]
pub fn Icon(icon: Icons, #[prop(into, optional)] class: Option<TextProp>) -> impl IntoView {
    view! {
        <span class=class inner_html=icon.svg()>
        </span>
    }
}
