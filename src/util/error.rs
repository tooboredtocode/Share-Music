/*
 * Copyright (c) 2021-2025 tooboredtocode
 * All Rights Reserved
 */

pub type EmptyResult<T> = Result<T, ()>;

#[derive(Debug, Clone)]
pub struct ExpectErr;

impl std::fmt::Display for ExpectErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("An error occurred, see logs for details")
    }
}

impl std::error::Error for ExpectErr {}

impl From<ExpectErr> for () {
    fn from(_: ExpectErr) -> Self {
        ()
    }
}

macro_rules! expect_err {
    ($($args:tt)*) => {
        |err| {
            tracing::error!(failed_with = %err, $($args)*);
            $crate::util::error::ExpectErr
        }
    };
}

pub(crate) use expect_err;

macro_rules! expect_warn {
    ($($args:tt)*) => {
        |err| {
            tracing::warn!(failed_with = %err, $($args)*);
            $crate::util::error::ExpectErr
        }
    };
}

pub(crate) use expect_warn;
