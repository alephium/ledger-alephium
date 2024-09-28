#[cfg(not(any(target_os = "stax", target_os = "flex")))]
pub mod multi_field_review;

#[cfg(any(target_os = "stax", target_os = "flex"))]
pub mod nbgl_display;
