use leptos::*;

use cfg_if::cfg_if;

#[cfg_attr(
    any(debug_assertions, feature = "ssr"),
    tracing::instrument(level = "trace", skip_all)
)]
#[component]
pub fn Portal(
    /// Target element where the children will be appended
    #[prop(into, optional)]
    mount: Option<web_sys::Element>,
    /// Whether to use a shadow DOM inside `mount`. Defaults to `false`.
    #[prop(optional)]
    use_shadow: bool,
    /// When using SVG this has to be set to `true`. Defaults to `false`.
    #[prop(optional)]
    is_svg: bool,
    /// The children to teleport into the `mount` element
    children: ChildrenFn,

    #[prop(optional, into)] mount_class: Option<String>,

    #[prop(optional, into)] class: Option<String>,
) -> impl IntoView {
    cfg_if! { if #[cfg(all(target_arch = "wasm32", any(feature = "hydrate", feature = "csr")))] {
        use leptos_dom::{document, Mountable};
        use wasm_bindgen::JsCast;

        let mount = mount
            .unwrap_or_else(|| document().body().expect("body to exist").unchecked_into());

        create_effect(move |_| {
            let tag = if is_svg { "g" } else { "div" };

            let container = document()
                .create_element(tag)
                .expect("element creation to work");
            if let Some(class) = &class{
                container.set_class_name(&class);
            }

            let render_root = if use_shadow {
                container
                    .attach_shadow(&web_sys::ShadowRootInit::new(
                        web_sys::ShadowRootMode::Open,
                    ))
                    .map(|root| root.unchecked_into())
                    .unwrap_or(container.clone())
            } else {
                container.clone()
            };

            let children = untrack(|| children().into_view().get_mountable_node());
            let _ = render_root.append_child(&children);

            let _ = mount.append_child(&container);

            let mut original_mount_class = None;
            if let Some(mount_class) = &mount_class {
                // tracing::info!("mount  class {}", mount.class_name());
                original_mount_class = Some(mount.class_name());
                mount.set_class_name(mount_class);
            }

            on_cleanup({
                let mount = mount.clone();
                move || {
                    let _ = mount.remove_child(&container);
                    if let Some(class) = original_mount_class {
                        mount.set_class_name(&class);
                    }
                }
            })
        });
    } else {
        let _ = mount;
        let _ = use_shadow;
        let _ = is_svg;
        let _ = children;
    }}
}
