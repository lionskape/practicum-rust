use web_sys::window;

const TOKEN_KEY: &str = "blog_token";
const USER_ID_KEY: &str = "blog_user_id";

pub fn save_token(token: &str) {
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(TOKEN_KEY, token);
    }
}

pub fn load_token() -> Option<String> {
    local_storage()?.get_item(TOKEN_KEY).ok()?
}

pub fn save_user_id(id: i64) {
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(USER_ID_KEY, &id.to_string());
    }
}

pub fn load_user_id() -> Option<i64> {
    local_storage()?.get_item(USER_ID_KEY).ok()??.parse().ok()
}

pub fn remove_token() {
    if let Some(storage) = local_storage() {
        let _ = storage.remove_item(TOKEN_KEY);
        let _ = storage.remove_item(USER_ID_KEY);
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok()?
}
