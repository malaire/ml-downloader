#![doc = include_str!(concat!(env!("OUT_DIR"), "/README-rustdocified.md"))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub use crate::{
    downloader::Downloader,
    downloader_builder::DownloaderBuilder,
    error::Error,
    request_builder::RequestBuilder,
    response::{GetResponse, SaveToFileResponse},
};

mod downloader;
mod downloader_builder;
mod error;
mod request_builder;
mod response;
mod util;
