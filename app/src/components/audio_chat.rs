use std::collections::HashMap;

use leptos::*;
use leptos_use::use_raf_fn;
use logging::warn;
use tracing::info;
use uuid::Uuid;
use web_sys::AudioContext;

use crate::components::portal::Portal;
use crate::components::video_chat::VideoChatManager;
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
    let user_ids = create_memo(move |_| users.get().into_iter().map(|u| u.id).collect::<Vec<_>>());

    let acs = store_value(HashMap::new());

    let owner = Owner::current().unwrap();

    create_effect(move |_| {
        if let Some((user_id, stream)) = audio_receiver.get() {
            if let Some(stream) = stream {
                if let Some(audio_ref) = audio_tag_ref.with(|map| map.get(&user_id).cloned()) {
                    if let Some(audio) = audio_ref.get_untracked() {
                        audio.set_src_object(Some(&stream));
                        info!("Playing audio");
                        if let Err(err) = audio.play() {
                            warn!("Cannot play audio {err:?}")
                        }
                    } else {
                        info!("No audio in ref");
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

                            analyzer.set_fft_size(2048);
                            let buffer_length = analyzer.fft_size();
                            let buffer = with_owner(owner, || {
                                store_value(vec![0_u8; buffer_length as usize])
                            });

                            with_owner(owner, || {
                                use_raf_fn(move |_| {
                                    buffer.update_value(|buffer| {
                                        analyzer.get_byte_time_domain_data(buffer);
                                        let sum_of_squares: f64 = buffer
                                            .iter()
                                            .map(|&val| {
                                                let normalized = (f64::from(val) - 128.0) / 128.0;
                                                normalized * normalized
                                            })
                                            .sum();

                                        let rms = ((sum_of_squares / f64::from(buffer_length))
                                            + 1e-10)
                                            .sqrt();

                                        // Map dB to percentage, assuming typical values for speech
                                        // Adjust these values based on your specific use case
                                        const MIN_DB: f64 = -70.0; // Adjust this if needed
                                        const MAX_DB: f64 = 0.0; // 0 dB represents maximum volume

                                        // Convert RMS to decibels, then to percentage
                                        // Convert RMS to decibels, then to percentage
                                        let db = if rms > 0.0 {
                                            20.0 * rms.log10()
                                        } else {
                                            MIN_DB
                                        };

                                        let volume_percentage = ((db - MIN_DB) / (MAX_DB - MIN_DB)
                                            * 100.0)
                                            .clamp(0.0, 100.0);

                                        // info!("Volume {user_id} {volume_percentage:00?}");

                                        progress_div_ref.update(|prog_map| {
                                            prog_map.insert(user_id, Some(volume_percentage));
                                        });
                                    });
                                });
                            });
                            acs.update_value(|acs| {
                                acs.insert(user_id, ac);
                            });
                        }
                        Err(err) => warn!("Cant create audio context {err:?}"),
                    }
                }
            } else {
                info!("Remove audio");
                if let Some(audio_ref) = audio_tag_ref.with(|map| map.get(&user_id).cloned()) {
                    if let Some(audio) = audio_ref.get_untracked() {
                        audio.set_src_object(None);
                    }
                }
                acs.update_value(|acs| {
                    if let Some(ac) = acs.remove(&user_id) {
                        progress_div_ref.update(|prog_map| {
                            prog_map.remove(&user_id);
                        });

                        let _ = ac.close();
                    }
                });
            }
        }
    });

    create_effect(move |_| {
        for user in user_ids.get() {
            audio_tag_ref.update(move |tag_map| {
                tag_map.entry(user).or_insert(with_owner(owner, || {
                    create_node_ref::<leptos::html::Audio>()
                }));
            });
        }
    });

    let (video_manager_open, set_video_manager_open) = create_signal(false);

    view! {
        <VideoChatManager
            is_open=video_manager_open
            close=move|_|{
                set_video_manager_open.set(false);
            }
        />
        {
            move || {
                if let Some(speaker_point) = speaker_point.get() {
                    let el:&web_sys::Element = speaker_point.as_ref();

                    view! {
                        <Portal
                            mount=el.clone()
                            class="h-full w-full bg-black p-2 flex justify-center flex-col"
                        >
                            <button class="text-xs text-center"
                                on:click=move|_|{
                                    set_video_manager_open.set(true);
                                }
                            >
                                "Video Call"
                            </button>
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

                                            let rm = expect_context::<RoomManager>();

                                            let (audio_ref, set_audio_ref)= create_signal(None);
                                            create_effect(move|_|{
                                                if audio_ref.get().is_none(){
                                                    if let Some(tag_ref) =  audio_tag_ref.with(|m|m.get(&user.id).cloned()){
                                                        set_audio_ref.set(Some(tag_ref))
                                                    }
                                                }
                                            });
                                            let self_user_if = rm.get_room_info().with(|r|r.as_ref().map(|u|u.user_id));

                                            view! {
                                                {
                                                    move || {
                                                        if Some(user.id) != self_user_if   {
                                                            if let Some(audio_ref) = audio_ref.get() {
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
