use gl::types::GLchar;


pub unsafe fn get_parameter_string(parameter: u32) -> String {
    let raw_ptr = gl::GetString(parameter);
    if raw_ptr.is_null() {
        panic!(
            "Get parameter string 0x{:X} failed. Maybe your GL context version is too outdated.",
            parameter
        )
    }
    std::ffi::CStr::from_ptr(raw_ptr as *const GLchar)
        .to_str()
        .unwrap()
        .to_owned()
}

pub unsafe fn get_parameter_i32(parameter: u32) -> i32 {
    let mut value = 0;
    gl::GetIntegerv(parameter, &mut value);
    value
}
