use leptos::*;
use tracing::info;

use crate::{components::portal::Portal, MountPoints};

#[component]
pub fn ChatBox() -> impl IntoView {
    view! {
        {
            move || {
                    let mount_points = expect_context::<MountPoints>();
                    let el = mount_points.side_point.get();
                    if let Some(el) = el {
                        let element: &web_sys::Element = el.as_ref();
                        let element = element.clone();

                        info!("Mounting to portal");
                        view! {
                            <Portal
                                mount=element
                                mount_class="h-full w-full"
                                class="h-full w-full flex flex-col justify-stretch"
                            >
                                <div class="text-center w-full"> "Chat" </div>
                                // <hr class="border-white border-t w-full" />

                            </Portal>
                        }.into_view()
                    }else{
                        info!("Side point not mounted");
                        view! {}.into_view()
                    }
            }
        }
    }
}
