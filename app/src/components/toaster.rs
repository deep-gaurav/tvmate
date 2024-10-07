use std::borrow::Cow;

use leptos::*;
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};

use crate::components::{dialog::Dialog, icons::Icon};

#[derive(Clone)]
pub enum ToastType {
    Success,
    Failed,
    Info,
}

#[derive(Clone)]
pub struct Toast {
    pub message: Cow<'static, str>,
    pub r#type: ToastType,
}

#[derive(Clone, Copy)]
pub struct Toaster {
    write_toast: WriteSignal<Option<Toast>>,
}

impl Toaster {
    pub fn toast(&self, toast: Toast) {
        self.write_toast.set(Some(toast));
    }
}

#[component]
pub fn ToasterWrapper(children: Children) -> impl IntoView {
    let (toast_rx, toast_tx) = create_signal(None);

    provide_context(Toaster {
        write_toast: toast_tx,
    });

    let UseTimeoutFnReturn { start, stop, .. } = use_timeout_fn(
        move |_: ()| {
            toast_tx.set(None);
        },
        3000.0,
    );

    create_effect(move |_| {
        if let Some(toast) = toast_rx.get() {
            stop();

            match toast.r#type {
                ToastType::Success => {
                    start(());
                }
                ToastType::Failed => {}
                ToastType::Info => {
                    start(());
                }
            }
        }
    });

    view! {
        {
            move || {
                if let Some(toast) = toast_rx.get(){
                    let animated_in = create_rw_signal(false);

                    let UseTimeoutFnReturn{start,..}= use_timeout_fn(move|_:()|{
                        request_animation_frame(move||{
                            animated_in.set(true);
                        });
                    }, 0.0);
                    start(());

                    view! {
                        <div class="fixed w-fit h-fit bottom-6 z-50 text-white transition-all duration-200 right-6 -translate-x-full"
                            class=("translate-x-full", move|| !animated_in.get())
                            class=("translate-x-0", move|| animated_in.get())
                        >
                        <Dialog
                            is_self_sized=true
                            is_open=true
                            on_close=move|_|{
                                toast_tx.set(None);
                            }
                        >
                            <div class="font-thin8 text-lg
                                flex flex-row gap-2 items-center
                            "
                            >
                                {
                                    match toast.r#type {
                                        ToastType::Success => view! {
                                            <Icon class="w-8 text-green-500" icon=crate::components::icons::Icons::Tick />
                                        },
                                        ToastType::Failed => view! {
                                            <Icon class="w-8 text-red-500" icon=crate::components::icons::Icons::Close />
                                        },
                                        ToastType::Info => view! {
                                            <Icon class="w-8 text-white" icon=crate::components::icons::Icons::Info />
                                        },
                                    }
                                }
                                {toast.message}
                            </div>
                        </Dialog>
                        </div>
                    }.into_view()
                }else{
                    view! {}.into_view()
                }
            }
        }
        {children()}
    }
}
