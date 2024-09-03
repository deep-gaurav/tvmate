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
                        <div class="h-full w-full flex items-center justify-center flex-col">
                            <div class="h-4" />
                            <h1 class="text-xl font-bold2"> "Room " {room_id.to_uppercase()} </h1>
                            <div class="h-full p-4 flex flex-col items-center justify-center">
                                <div class="h-4" />
                                <label for="video-picker"> "Click here to pick Video" </label>
                                <input class="hidden" type="file" id="video-picker" accept="video/*"/>
                            </div>
                        </div>
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
