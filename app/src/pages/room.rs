use std::ops::Deref;

use leptos::*;
use leptos_meta::Title;
use leptos_router::*;
use tracing::info;
use wasm_bindgen::{JsCast, UnwrapThrowExt};

use crate::{
    components::{portal::Portal, room_info::RoomInfo, video_player::VideoPlayer},
    MountPoints,
};

#[derive(Params, PartialEq, Clone)]
struct RoomParam {
    id: Option<String>,
}
#[component]
pub fn RoomPage() -> impl IntoView {
    let params = use_params::<RoomParam>();
    let (video_url, set_video_url) = create_signal(None);

    view! {
        {
            move || if let Ok(RoomParam { id: Some(room_id) }) = params.get() {
                if !room_id.is_empty() {
                    view! {
                        <Title text=format!("Room {room_id}")/>
                        <VideoPlayer src=video_url />
                        <RoomInfo />
                        <div class="h-full w-full flex px-10 py-4 items-center justify-center flex-col"
                            class=("hidden",move || video_url.with(|v|v.is_some()))
                        >
                            <div class="h-4" />
                            <h1 class="text-xl font-bold2"> "Room " {room_id.to_uppercase()} </h1>

                            <div
                                class="h-full w-full my-8 p-4 flex flex-col items-center justify-center border-white border-dotted border-2 rounded-sm"
                                >
                                <div class="h-4" />
                                <label for="video-picker" class="flex flex-col items-center justify-center">
                                    <div>"Drag and Drop Video"</div>
                                        <div>"Or"</div>
                                    <div>"Click here to pick"</div>
                                </label>
                                <input class="hidden" type="file" id="video-picker" accept="video/*"
                                    on:change=move|ev| {
                                        let input_el = ev
                                        .unchecked_ref::<web_sys::Event>()
                                        .target()
                                        .unwrap_throw()
                                        .unchecked_into::<web_sys::HtmlInputElement>();
                                        let files = input_el.files();
                                        if let Some(file) = files.and_then(|f|f.item(0)) {
                                            let blob = file.unchecked_ref::<web_sys::Blob>();
                                            info!("Name: {}, Type: {}", file.name(), blob.type_());
                                            let url = web_sys::Url::create_object_url_with_blob(blob);
                                            info!("Video URL {url:#?}");
                                            if let Ok(url) = url {
                                                set_video_url.set(Some(url));
                                            }
                                        }
                                    }
                                />
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
