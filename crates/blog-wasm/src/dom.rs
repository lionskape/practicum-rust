use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlInputElement, HtmlTextAreaElement, window};

use crate::{api, storage};

pub fn document() -> Document {
    window().expect("no window").document().expect("no document")
}

pub fn get_input_value(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

pub fn get_textarea_value(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|ta| ta.value())
        .unwrap_or_default()
}

pub fn set_inner_html(id: &str, html: &str) {
    if let Some(el) = document().get_element_by_id(id) {
        el.set_inner_html(html);
    }
}

pub fn create_element(tag: &str) -> Element {
    document().create_element(tag).expect("failed to create element")
}

pub fn show_message(id: &str, msg: &str, is_error: bool) {
    let color = if is_error { "red" } else { "green" };
    set_inner_html(id, &format!("<span style='color:{color}'>{msg}</span>"));
}

pub fn render_app() {
    let doc = document();
    let body = doc.body().expect("no body");

    let is_logged_in = storage::load_token().is_some();

    let html = if is_logged_in {
        r#"<div style="max-width:800px;margin:0 auto;padding:20px;font-family:sans-serif">
            <h1>Blog Platform</h1>
            <div id="status"></div>
            <button id="btn-logout">Logout</button>
            <hr>
            <h2>Create Post</h2>
            <input id="post-title" placeholder="Title" style="width:100%;margin:5px 0;padding:8px">
            <textarea id="post-content" placeholder="Content" rows="4" style="width:100%;margin:5px 0;padding:8px"></textarea>
            <button id="btn-create">Create Post</button>
            <div id="create-msg"></div>
            <hr>
            <div id="edit-section" style="display:none;border:2px solid #4a90d9;padding:12px;margin:8px 0;border-radius:4px;background:#eef4fb">
                <h2>Edit Post <span id="edit-post-id"></span></h2>
                <input id="edit-title" placeholder="Title" style="width:100%;margin:5px 0;padding:8px">
                <textarea id="edit-content" placeholder="Content" rows="4" style="width:100%;margin:5px 0;padding:8px"></textarea>
                <button id="btn-save-edit">Save</button>
                <button id="btn-cancel-edit">Cancel</button>
                <div id="edit-msg"></div>
            </div>
            <h2>Posts</h2>
            <button id="btn-refresh">Refresh</button>
            <div id="posts-list"></div>
            </div>"#
    } else {
        r#"<div style="max-width:400px;margin:50px auto;padding:20px;font-family:sans-serif">
            <h1>Blog Platform</h1>
            <div id="status"></div>
            <h2>Register</h2>
            <input id="reg-username" placeholder="Username" style="width:100%;margin:5px 0;padding:8px">
            <input id="reg-email" placeholder="Email" style="width:100%;margin:5px 0;padding:8px">
            <input id="reg-password" type="password" placeholder="Password" style="width:100%;margin:5px 0;padding:8px">
            <button id="btn-register">Register</button>
            <div id="reg-msg"></div>
            <hr>
            <h2>Login</h2>
            <input id="login-username" placeholder="Username" style="width:100%;margin:5px 0;padding:8px">
            <input id="login-password" type="password" placeholder="Password" style="width:100%;margin:5px 0;padding:8px">
            <button id="btn-login">Login</button>
            <div id="login-msg"></div>
            </div>"#
    };

    body.set_inner_html(html);
    setup_handlers(is_logged_in);
}

fn setup_handlers(is_logged_in: bool) {
    let doc = document();

    if is_logged_in {
        if let Some(btn) = doc.get_element_by_id("btn-logout") {
            let cb = Closure::wrap(Box::new(move || {
                storage::remove_token();
                render_app();
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }

        if let Some(btn) = doc.get_element_by_id("btn-create") {
            let cb = Closure::wrap(Box::new(move || {
                let title = get_input_value("post-title");
                let content = get_textarea_value("post-content");
                wasm_bindgen_futures::spawn_local(async move {
                    match api::create_post(&title, &content).await {
                        Ok(post) => {
                            show_message(
                                "create-msg",
                                &format!("Post '{}' created!", post.title),
                                false,
                            );
                            load_posts().await;
                        }
                        Err(e) => show_message("create-msg", &format!("{e:?}"), true),
                    }
                });
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }

        if let Some(btn) = doc.get_element_by_id("btn-refresh") {
            let cb = Closure::wrap(Box::new(move || {
                wasm_bindgen_futures::spawn_local(async move {
                    load_posts().await;
                });
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }

        if let Some(btn) = doc.get_element_by_id("btn-save-edit") {
            let cb = Closure::wrap(Box::new(move || {
                let id_str = document()
                    .get_element_by_id("edit-section")
                    .and_then(|el| el.get_attribute("data-editing-id"))
                    .unwrap_or_default();
                let post_id: i64 = id_str.parse().unwrap_or(0);
                let title = get_input_value("edit-title");
                let content = get_textarea_value("edit-content");
                wasm_bindgen_futures::spawn_local(async move {
                    match api::update_post(post_id, &title, &content).await {
                        Ok(post) => {
                            show_message(
                                "edit-msg",
                                &format!("Post '{}' updated!", post.title),
                                false,
                            );
                            hide_edit_form();
                            load_posts().await;
                        }
                        Err(e) => show_message("edit-msg", &format!("{e:?}"), true),
                    }
                });
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }

        if let Some(btn) = doc.get_element_by_id("btn-cancel-edit") {
            let cb = Closure::wrap(Box::new(move || {
                hide_edit_form();
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }

        wasm_bindgen_futures::spawn_local(async move {
            load_posts().await;
        });
    } else {
        if let Some(btn) = doc.get_element_by_id("btn-register") {
            let cb = Closure::wrap(Box::new(move || {
                let username = get_input_value("reg-username");
                let email = get_input_value("reg-email");
                let password = get_input_value("reg-password");
                wasm_bindgen_futures::spawn_local(async move {
                    match api::register(&username, &email, &password).await {
                        Ok(auth) => {
                            show_message(
                                "reg-msg",
                                &format!("Welcome, {}!", auth.user.username),
                                false,
                            );
                            render_app();
                        }
                        Err(e) => show_message("reg-msg", &format!("{e:?}"), true),
                    }
                });
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }

        if let Some(btn) = doc.get_element_by_id("btn-login") {
            let cb = Closure::wrap(Box::new(move || {
                let username = get_input_value("login-username");
                let password = get_input_value("login-password");
                wasm_bindgen_futures::spawn_local(async move {
                    match api::login(&username, &password).await {
                        Ok(auth) => {
                            show_message(
                                "login-msg",
                                &format!("Welcome back, {}!", auth.user.username),
                                false,
                            );
                            render_app();
                        }
                        Err(e) => show_message("login-msg", &format!("{e:?}"), true),
                    }
                });
            }) as Box<dyn Fn()>);
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
            cb.forget();
        }
    }
}

async fn load_posts() {
    let current_user_id = storage::load_user_id();

    match api::list_posts(20, 0).await {
        Ok(resp) => {
            let mut html = String::new();
            if resp.posts.is_empty() {
                html.push_str("<p>No posts yet.</p>");
            }
            for post in &resp.posts {
                let is_author = current_user_id.is_some_and(|uid| uid == post.author_id);
                let actions = if is_author {
                    format!(
                        r#"<button class="btn-edit" data-id="{}" data-title="{}" data-content="{}">Edit</button>
                        <button class="btn-delete" data-id="{}">Delete</button>"#,
                        post.id,
                        html_escape(&post.title),
                        html_escape(&post.content),
                        post.id,
                    )
                } else {
                    String::new()
                };
                html.push_str(&format!(
                    r#"<div style="border:1px solid #ccc;padding:12px;margin:8px 0;border-radius:4px">
                    <h3>{}</h3>
                    <p>{}</p>
                    <small>by {} | {}</small>
                    <div style="margin-top:8px">{actions}</div>
                    </div>"#,
                    post.title, post.content, post.author_username, post.created_at
                ));
            }
            html.push_str(&format!("<p><em>Total: {}</em></p>", resp.total));
            set_inner_html("posts-list", &html);
            attach_post_handlers();
        }
        Err(e) => {
            set_inner_html("posts-list", &format!("<p style='color:red'>Error: {e:?}</p>"));
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

fn attach_post_handlers() {
    let doc = document();

    let delete_buttons = doc.query_selector_all(".btn-delete").ok();
    if let Some(buttons) = delete_buttons {
        for i in 0..buttons.length() {
            if let Some(btn) = buttons.item(i) {
                let el: &Element = btn.unchecked_ref();
                if let Some(id_str) = el.get_attribute("data-id") {
                    let post_id: i64 = id_str.parse().unwrap_or(0);
                    let cb = Closure::wrap(Box::new(move || {
                        wasm_bindgen_futures::spawn_local(async move {
                            match api::delete_post(post_id).await {
                                Ok(()) => load_posts().await,
                                Err(e) => {
                                    show_message("create-msg", &format!("{e:?}"), true);
                                }
                            }
                        });
                    }) as Box<dyn Fn()>);
                    el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
                    cb.forget();
                }
            }
        }
    }

    let edit_buttons = doc.query_selector_all(".btn-edit").ok();
    if let Some(buttons) = edit_buttons {
        for i in 0..buttons.length() {
            if let Some(btn) = buttons.item(i) {
                let el: &Element = btn.unchecked_ref();
                let id_str = el.get_attribute("data-id").unwrap_or_default();
                let title = el.get_attribute("data-title").unwrap_or_default();
                let content = el.get_attribute("data-content").unwrap_or_default();
                let cb = Closure::wrap(Box::new(move || {
                    show_edit_form(&id_str, &title, &content);
                }) as Box<dyn Fn()>);
                el.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref()).ok();
                cb.forget();
            }
        }
    }
}

fn show_edit_form(id: &str, title: &str, content: &str) {
    let doc = document();
    if let Some(section) = doc.get_element_by_id("edit-section") {
        section
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("display", "block")
            .ok();
    }
    set_inner_html("edit-post-id", &format!("#{id}"));
    if let Some(el) = doc.get_element_by_id("edit-title") {
        el.unchecked_ref::<HtmlInputElement>().set_value(title);
    }
    if let Some(el) = doc.get_element_by_id("edit-content") {
        el.unchecked_ref::<HtmlTextAreaElement>().set_value(content);
    }
    if let Some(el) = doc.get_element_by_id("edit-section") {
        el.set_attribute("data-editing-id", id).ok();
    }
}

fn hide_edit_form() {
    if let Some(section) = document().get_element_by_id("edit-section") {
        section
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("display", "none")
            .ok();
    }
}
