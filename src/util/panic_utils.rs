use std::any::Any;

pub fn panic_payload_as_str(payload: &(dyn Any + Send)) -> Option<&str> {
    if let Some(s) = payload.downcast_ref::<String>() {
        return Some(s);
    }

    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return Some(s);
    }

    None
}
