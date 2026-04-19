use std::cell::RefCell;

thread_local! {
    static REQUEST_ID: RefCell<Option<String>> = RefCell::new(None);
}

pub fn set_request_id(id: String) {
    REQUEST_ID.with(|rid| {
        *rid.borrow_mut() = Some(id);
    });
}

pub fn get_request_id() -> Option<String> {
    REQUEST_ID.with(|rid| rid.borrow().clone())
}

pub fn clear_request_id() {
    REQUEST_ID.with(|rid| {
        *rid.borrow_mut() = None;
    });
}
