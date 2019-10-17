macro_rules! file_descriptor_type {
    ($type:ident) => {
        #[repr(transparent)]
        pub struct $type(RawFd);

        file_descriptor_impl!($type);

        impl FromRawFd for $type {
            unsafe fn from_raw_fd(fd: RawFd) -> Self {
                Self(fd)
            }
        }
    };
}

macro_rules! file_descriptor_impl {
    ($type:ty) => {
        impl Drop for $type {
            fn drop(&mut self) {
                if self.0 >= 0 {
                    unsafe {
                        libc::close(self.0);
                    }
                }
            }
        }

        impl AsRawFd for $type {
            fn as_raw_fd(&self) -> RawFd {
                self.0
            }
        }

        impl IntoRawFd for $type {
            fn into_raw_fd(mut self) -> RawFd {
                let fd = self.0;
                self.0 = -libc::EBADF;
                fd
            }
        }
    };
}

macro_rules! c_call {
    ($expr:expr) => {{
        let res = $expr;
        if res == -1 {
            Err(::std::io::Error::last_os_error())
        } else {
            Ok::<_, ::std::io::Error>(res)
        }
    }};
}

macro_rules! c_try {
    ($expr:expr) => {
        c_call!($expr)?
    };
}

macro_rules! io_format_err {
    ($($msg:tt)*) => {
        ::std::io::Error::new(::std::io::ErrorKind::Other, format!($($msg)*))
    };
}

macro_rules! io_bail {
    ($($msg:tt)*) => {
        return Err(::std::io::Error::new(::std::io::ErrorKind::Other, format!($($msg)*)));
    };
}
