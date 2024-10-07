use leptos::*;

pub enum Icons {
    Info,
    Tick,
    Close,
}

impl Icons {
    pub fn svg(&self) -> &'static str {
        match self {
            Icons::Info => include_str!("info.svg"),
            Icons::Tick => include_str!("tick.svg"),
            Icons::Close => include_str!("close.svg"),
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
