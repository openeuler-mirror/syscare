#[macro_export]
macro_rules! concat_os {
    ($($e:expr),* $(,)?) => {{
        use std::ffi::OsString;

        let mut buf = OsString::new();
        $(
            buf.push($e);
        )*
        buf
    }};
}

#[macro_export]
macro_rules! args_os {
    ($($e:expr),* $(,)?) => {{
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};

        let mut buf = OsString::new();
        $(
            buf.push($e);
            buf.push(" ");
        )*
        let mut vec = buf.into_vec();
        vec.pop();

        OsString::from_vec(vec)
    }};
}

#[test]
fn test() {
    let std_str =
        concat!("The ", "quick ", "brown ", "fox ", "jumps ", "over ", "a ", "lazy ", "dog");
    let os_str =
        concat_os!("The ", "quick ", "brown ", "fox ", "jumps ", "over ", "a ", "lazy ", "dog");
    let arg_str = args_os!("The", "quick", "brown", "fox", "jumps", "over", "a", "lazy", "dog");

    assert_eq!(std_str, os_str);
    assert_eq!(std_str, arg_str);
}
