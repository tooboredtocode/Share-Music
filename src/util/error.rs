/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

pub type EmptyResult<T> = Result<T, ()>;

macro_rules! expect_err {
    ($($args:tt)*) => {
        |err| {
            tracing::error!(failed_with = %err, $($args)*);
        }
    };
}

pub(crate) use expect_err;

macro_rules! expect_warn {
    ($($args:tt)*) => {
        |err| {
            tracing::warn!(failed_with = %err, $($args)*);
        }
    };
}

pub(crate) use expect_warn;
