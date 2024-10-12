use leptos::*;
use leptos_meta::{Meta, Title};
use leptos_router::*;
use tracing::info;
use wasm_bindgen::{JsCast, UnwrapThrowExt};

use crate::{
    apis::get_room_info,
    components::{
        audio_chat::AudioChat, chatbox::ChatBox, join_dialog::JoinDialog, room_info::RoomInfo,
        video_chat::VideoChat, video_player::VideoPlayer,
    },
    networking::room_manager::RoomManager,
};

#[derive(Params, PartialEq, Clone)]
struct RoomParam {
    id: Option<String>,
}
#[component]
pub fn RoomPage() -> impl IntoView {
    let params = use_params::<RoomParam>();
    let (video_url, set_video_url) = create_signal(None);
    let (video_name, set_video_name) = create_signal(None);

    let room_manager = expect_context::<RoomManager>();
    create_effect({
        let room_manager = room_manager.clone();
        move |_| {
            if let Some(video_name) = video_name.get() {
                room_manager.set_selected_video(video_name);
            }
        }
    });

    let (is_csr, set_is_csr) = create_signal(false);
    create_effect(move |_| set_is_csr.set(true));

    let (join_dialog_open, set_join_dialog_open) = create_signal(false);

    let room_info = room_manager.get_room_info();
    create_effect(move |_| {
        if room_info.with(|r| r.is_none()) {
            set_join_dialog_open.set(true);
        } else {
            set_join_dialog_open.set(false);
        }
    });

    let room_meta = create_blocking_resource(
        move || params.get().map(|param| param.id).ok().flatten(),
        |room_id| async move {
            if let Some(room_id) = room_id {
                get_room_info(room_id).await
            } else {
                Err(ServerFnError::new("room id doesnt exist"))
            }
        },
    );

    view! {
        <JoinDialog
            is_open=join_dialog_open
            on_close=Callback::new(move|_|{
                set_join_dialog_open.set(false);
            })
            init_room_code={
                create_memo(move |_| {
                    if let Ok(RoomParam { id: Some(room_id) }) = params.get() {
                        info!("Room id {room_id}");
                        room_id
                    }else{
                        info!("Room id empty");
                        "".to_string()
                    }
                })
            }
        />
        <Suspense>
            {
                move || if let Some(Ok(Some(room_meta))) = room_meta.get(){
                    view! {
                        <Title text=format!("TVMate | {}", &room_meta.room_id) />
                        <Meta property="og:title" content=format!("TVMate | {}", &room_meta.room_id) />
                        <Meta property="og:description" content=format!("Join {} to have watch party together", &room_meta.host) />
                        <meta property="og:type" content="website" />
                        <Meta name="description" content=format!("Join {} to have watch party together", &room_meta.host) />
                    }.into_view()
                }else{
                    view! {
                        <Meta name="description" content="Let's have watch party together" />
                    }.into_view()
                }
            }
        </Suspense>

        {move || {
            if let Ok(RoomParam { id: Some(room_id) }) = params.get() {
                if !room_id.is_empty() {

                    view! {
                        <VideoPlayer src=video_url />

                        {move || {
                            if is_csr.get() {
                                view! {
                                    <RoomInfo />
                                    <ChatBox />
                                    <AudioChat />
                                    <VideoChat />
                                }
                                    .into_view()
                            } else {
                                view! {}.into_view()
                            }
                        }}
                        <div
                            class="h-full w-full flex px-10 py-4 items-center justify-center flex-col"
                            class=("hidden", move || video_url.with(|v| v.is_some()))
                        >
                            <div class="h-4" />
                            <h1 class="text-xl font-bold2">"Room " {room_id.to_uppercase()}</h1>

                            <div class="h-full w-full my-8 p-4 flex flex-col items-center justify-center border-white border-dotted border-2 rounded-sm">
                                <div class="h-4" />
                                <label
                                    for="video-picker"
                                    class="flex flex-col items-center justify-center"
                                >
                                    <div>"Drag and Drop Video"</div>
                                    <div>"Or"</div>
                                    <div>"Click here to pick"</div>
                                </label>
                                <input
                                    class="hidden"
                                    type="file"
                                    id="video-picker"
                                    accept="video/*"
                                    on:change=move |ev| {
                                        let input_el = ev
                                            .unchecked_ref::<web_sys::Event>()
                                            .target()
                                            .unwrap_throw()
                                            .unchecked_into::<web_sys::HtmlInputElement>();
                                        let files = input_el.files();
                                        if let Some(file) = files.and_then(|f| f.item(0)) {
                                            let blob = file.unchecked_ref::<web_sys::Blob>();
                                            info!("Name: {}, Type: {}", file.name(), blob.type_());
                                            let url = web_sys::Url::create_object_url_with_blob(blob);
                                            info!("Video URL {url:#?}");
                                            if let Ok(url) = url {
                                                set_video_name.set(Some(file.name()));
                                                set_video_url.set(Some(url));
                                            }
                                        }
                                    }
                                />
                            </div>
                        </div>
                    }
                        .into_view()
                } else {
                    view! { <Redirect path="/" /> }.into_view()
                }
            } else {
                view! { <Redirect path="/" /> }.into_view()
            }
        }}

    }
}
