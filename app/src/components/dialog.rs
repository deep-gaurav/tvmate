use leptos::*;

#[component]
pub fn Dialog(
    #[prop(into)] is_open: MaybeSignal<bool>,
    #[prop(into)] on_close: Callback<()>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class="absolute h-full w-full bg-black/30 flex items-center justify-center p-10"
            class=("hidden", move || !is_open.get())
        >
            <div class="p-1 bg-black relative shadow-box shadow-white/45 ">
                <button
                    class=" absolute right-6 -top-1 bg-black text-sm font-bold1"
                    on:click=move |_| {
                        on_close.call(());
                    }
                >
                    " [ x ] "
                </button>
                <div class="border border-white p-0.5">
                    <div class="border border-white p-4 flex flex-col">{children()}</div>
                </div>
            </div>
        </div>
    }
}