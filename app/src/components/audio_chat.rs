use std::collections::HashMap;

use leptos::*;
use leptos_use::use_raf_fn;
use logging::warn;
use uuid::Uuid;
use web_sys::AudioContext;

use crate::components::portal::Portal;
use crate::networking::room_manager::RoomManager;
use crate::MountPoints;

#[component]
pub fn AudioChat() -> impl IntoView {
    let MountPoints { speaker_point, .. } = expect_context::<MountPoints>();

    let rm = expect_context::<RoomManager>();

    let audio_tag_ref = create_rw_signal(HashMap::<Uuid, NodeRef<leptos::html::Audio>>::new());
    let progress_div_ref = create_rw_signal(HashMap::<Uuid, Option<f64>>::new());

    let audio_receiver = rm.audio_chat_stream_signal.0;

    let users = create_memo({
        let rm = rm.clone();
        move |_| {
            if let Some(room_info) = rm.get_room_info().get() {
                room_info.users
            } else {
                vec![]
            }
        }
    });

    let owner = Owner::current().unwrap();

    create_effect(move |_| {
        if let Some((user_id, stream)) = audio_receiver.get() {
            if let Some(audio_ref) = audio_tag_ref.with(|map| map.get(&user_id).cloned()) {
                if let Some(audio) = audio_ref.get_untracked() {
                    audio.set_src_object(Some(&stream));
                    if let Err(err) = audio.play() {
                        warn!("Cannot play audio {err:?}")
                    }
                    let ac = AudioContext::new();
                    match ac {
                        Ok(ac) => {
                            let Ok(analyzer) = ac.create_analyser() else {
                                warn!("Cant create analyzer");
                                return;
                            };
                            let Ok(source) = ac.create_media_stream_source(&stream) else {
                                warn!("Cant create source node");
                                return;
                            };
                            if let Err(err) = source.connect_with_audio_node(&analyzer) {
                                warn!("cant connect {err:?}");
                            }

                            analyzer.set_fft_size(256);
                            let buffer_length = analyzer.frequency_bin_count();
                            let buffer = store_value(vec![0_u8; buffer_length as usize]);

                            with_owner(owner, || {
                                use_raf_fn(move |_| {
                                    buffer.update_value(|buffer| {
                                        analyzer.get_byte_frequency_data(buffer);
                                        let sum: f64 = buffer
                                            .iter()
                                            .map(|val| f64::from(*val).powf(2.0))
                                            .sum();
                                        let volume = ((sum / f64::from(buffer_length)).sqrt()
                                            / 256_f64)
                                            * 100_f64;
                                        progress_div_ref.update(|prog_map| {
                                            prog_map.insert(user_id, Some(volume));
                                        });
                                    });
                                });
                            });
                        }
                        Err(err) => warn!("Cant create audio context {err:?}"),
                    }
                }
            }
        }
    });

    create_effect(move |_| {
        for user in users.get() {
            audio_tag_ref.update(move |tag_map| {
                tag_map
                    .entry(user.id)
                    .or_insert(create_node_ref::<leptos::html::Audio>());
            });
        }
    });

    view! {
        {
            move || {
                if let Some(speaker_point) = speaker_point.get() {
                    let el:&web_sys::Element = speaker_point.as_ref();

                    view! {
                        <Portal
                            mount=el.clone()
                            class="h-full w-full bg-black p-2 flex justify-center flex-col"
                        >
                            <div class="text-xs "> "Audio Chat" </div>
                            <div class="h-4" />
                            <div class="flex flex-grow h-full w-full gap-2">
                                <For
                                    each=move||users.get()
                                    key=|user|user.id
                                    let:user
                                >
                                    <div class="h-full flex flex-col gap-2 justify-center">

                                        {
                                            let volumebar:NodeRef<leptos::html::Div> = create_node_ref();
                                            view! {
                                                <div ref=volumebar class="flex-grow flex gap-[2px] flex-col-reverse">
                                                {
                                                    move || {
                                                        if let Some(volume_bar) = volumebar.get(){
                                                            let height = volume_bar.offset_height();
                                                            let volume = progress_div_ref.with(|m|m.get(&user.id).map(|m|m.unwrap_or_default()).unwrap_or_default());
                                                            let bar_height = 2;
                                                            let gap = 2;
                                                            let max_bars = (height as f64)  /((gap + bar_height) as f64 );

                                                            let bars = ((volume/100.0)*max_bars) as u64;
                                                            view! {
                                                                {(0..bars).map(
                                                                    |_|{
                                                                        view! {
                                                                            <div class="w-full bg-green-600 h-[2px]" />
                                                                        }
                                                                    }
                                                                ).collect_view()}
                                                            }.into_view()
                                                        }else{
                                                            view! {

                                                            }.into_view()
                                                        }
                                                    }
                                                }
                                                </div>
                                            }
                                        }
                                        <div class="text-xs"> {user.name} </div>
                                        {
                                            move ||{
                                                let rm = expect_context::<RoomManager>();

                                                let audio_ref= audio_tag_ref.with(|m|m.get(&user.id).cloned());
                                                let self_user_if = rm.get_room_info().with(|r|r.as_ref().map(|u|u.user_id));

                                                if Some(user.id) != self_user_if   {
                                                    if let Some(audio_ref) = audio_ref {
                                                        view! {
                                                            <audio ref=audio_ref class="hidden" />
                                                        }.into_view()
                                                    }else{
                                                        view! {}.into_view()
                                                    }
                                                }else{
                                                    view! {}.into_view()
                                                }
                                            }
                                        }
                                    </div>
                                </For>
                            </div>
                        </Portal>
                    }.into_view()
                }
                else {
                    view! {

                    }.into_view()
                }
            }
        }
    }
}
