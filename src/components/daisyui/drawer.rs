use dioxus::prelude::*;

#[derive(Props, Clone, Debug, PartialEq)]
pub(crate) struct DrawerProps {
    trigger_id: String,
    label: String,

    #[props(optional)]
    class: String,
    #[props(optional)]
    drawer_class: String,

    #[props(optional)]
    page_content_class: String,
    page_content: Element,
    children: Element,
}

#[component]
pub fn Drawer(props: DrawerProps) -> Element {
    rsx!(
        div { class: "drawer {props.class}",
            input {
                id: props.trigger_id.clone(),
                r#type: "checkbox",
                class: "drawer-toggle",
            }
            div { class: "drawer-content {props.page_content_class}", {props.page_content} }
            div {
                id: "drawer-side-{props.trigger_id.clone()}",
                class: "drawer-side {props.drawer_class}",
                label {
                    r#for: props.trigger_id,
                    aria_label: "close sidebar",
                    class: "drawer-overlay",
                }
                {props.children}
            }
        }
    )
}
