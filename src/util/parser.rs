/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::{Body, body, Response};
use hyper::body::Bytes;
use image::{DynamicImage, ImageError};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub enum ParsingError {
    Chunking,
    DeserializingJson(serde_json::Error),
    DeserializingImage(ImageError)
}

impl Display for ParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            Self::Chunking => write!(f, "failed to chunk response body"),
            Self::DeserializingJson(err) => write!(f, "failed to deserialize response body: {}", err),
            Self::DeserializingImage(err) => write!(f, "failed to deserialize image: {}", err)
        }
    }
}

impl Error for ParsingError {}

pub struct BytesFuture {
    inner: Pin<Box<dyn Future<Output=Result<Bytes, ParsingError>> + Send + Sync + 'static>>,
}

impl BytesFuture {
    pub fn from_response(resp: Response<Body>) -> BytesFuture {
        let fut = async move {
            body::to_bytes(resp.into_body())
                .await
                .map_err(|_| ParsingError::Chunking)
        };

        BytesFuture {
            inner: Box::pin(fut),
        }
    }
}

impl Future for BytesFuture {
    type Output = Result<Vec<u8>, ParsingError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(result) = Pin::new(&mut self.inner).poll(cx) {
            Poll::Ready(result.map(|b| b.into_iter().collect()))
        } else {
            Poll::Pending
        }
    }
}

pub struct ParsingFuture<T> {
    future: BytesFuture,
    phantom: PhantomData<T>,
}

impl<T> ParsingFuture<T> {
    fn new(bytes: BytesFuture) -> Self {
        Self {
            future: bytes,
            phantom: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Unpin> Future for ParsingFuture<T> {
    type Output = Result<T, ParsingError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.future).poll(cx) {
            Poll::Ready(Ok(bytes)) => Poll::Ready(
                serde_json::from_slice(&Bytes::from(bytes))
                    .map_err(|source| ParsingError::DeserializingJson(source))
            ),
            Poll::Ready(Err(source)) => Poll::Ready(Err(source)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct ImageFuture {
    future: BytesFuture
}

impl ImageFuture {
    fn new(bytes: BytesFuture) -> Self {
        Self {
            future: bytes,
        }
    }
}

impl Future for ImageFuture {
    type Output = Result<DynamicImage, ParsingError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.future).poll(cx) {
            Poll::Ready(Ok(bytes)) => Poll::Ready(
                image::load_from_memory(&*bytes)
                    .map_err(|source| ParsingError::DeserializingImage(source))
            ),
            Poll::Ready(Err(source)) => Poll::Ready(Err(source)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn parse<T>(resp: Response<Body>) -> ParsingFuture<T> {
    ParsingFuture::new(BytesFuture::from_response(resp))
}

pub fn parse_image(resp: Response<Body>) -> ImageFuture {
    ImageFuture::new(BytesFuture::from_response(resp))
}