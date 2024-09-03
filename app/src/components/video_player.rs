use leptos::*;

#[component]
pub fn VideoPlayer(#[prop(into)] src: Signal<Option<String>>) -> impl IntoView {
    view! {
        <div
            class="h-full w-full relative"
            class=("hidden",move || src.with(|v|v.is_none()))
        >
            <video
                class="h-full w-full"
            >
                {
                    move || {
                        if let Some(url) = src.get(){
                            view! {
                                <source src=url />
                            }.into_view()
                        }else {
                            view! {}.into_view()
                        }
                    }
                }
            </video>
            <div class="absolute h-full w-full top-0 left-0 hover:bg-black/50">

            </div>
        </div>
    }
}
