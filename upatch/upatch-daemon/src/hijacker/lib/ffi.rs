extern "C" {
    pub fn upatch_hijacker_init() -> ::std::os::raw::c_int;
}

extern "C" {
    pub fn upatch_hijacker_cleanup() -> ::std::os::raw::c_void;
}

extern "C" {
    pub fn upatch_hijacker_register(
        prey_path: *const ::std::os::raw::c_char,
        hijacker_path: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}

extern "C" {
    pub fn upatch_hijacker_unregister(
        prey_path: *const ::std::os::raw::c_char,
        hijacker_path: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
