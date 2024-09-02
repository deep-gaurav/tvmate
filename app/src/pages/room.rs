use leptos::*;
use leptos_meta::Title;
use leptos_router::*;

#[derive(Params, PartialEq, Clone)]
struct RoomParam {
    id: Option<String>,
}
#[component]
pub fn RoomPage() -> impl IntoView {
    let params = use_params::<RoomParam>();
    view! {
        {
            move || if let Ok(RoomParam { id: Some(room_id) }) = params.get() {
                if !room_id.is_empty() {
                    view! {
                        <Title text=format!("Room {room_id}")/>

                    }.into_view()
                }else{
                    view! {
                        <Redirect path="/" />
                    }.into_view()
                }
            } else {
                view! {
                    <Redirect path="/" />
                }.into_view()
            }
        }
    }
}
