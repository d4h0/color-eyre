
//! Nothing interesting here. This is just a small helper used in a test.

//! This needs to be an "example" until binaries can declare separate dependencies (see https://github.com/rust-lang/cargo/issues/1982)

//! See "tests/styles.rs" for more information.

use color_eyre::{eyre::Report, Section};
use tracing_error::ErrorLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

#[rustfmt::skip]
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct TestError(&'static str);

#[rustfmt::skip]
#[tracing::instrument]
fn get_error(msg: &'static str) -> Report {

    #[rustfmt::skip]
    #[inline(never)]
    fn create_report(msg: &'static str) -> Report {
        Report::msg(msg)
            .note("note")
            .warning("warning")
            .suggestion("suggestion")
            .error(TestError("error"))
    }

    // Getting regular `Report`. Using `Option` to trigger `is_dependency_code`. See https://github.com/yaahc/color-eyre/blob/4ddaeb2126ed8b14e4e6aa03d7eef49eb8561cf0/src/config.rs#L56
    None::<Option<()>>.ok_or_else(|| create_report(msg)).unwrap_err()
}

fn main(){
    setup();
    let error = get_error("test");
    panic!(error)
}

fn setup() {
    std::env::set_var("RUST_LIB_BACKTRACE", "full");

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();

    color_eyre::install().expect("Failed to install `color_eyre`");
}