fn main() -> iced::Result {
    // Desktop-only binary: Android starts through `android_main`, where
    // `android_logger` is installed instead. The original app printed entry and
    // diagnostic lines to the console with `debugPrint`, so mirror that on
    // stdout here. `RUST_LOG` overrides the default.
    #[cfg(not(target_os = "android"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,zipp=info"))
        .target(env_logger::Target::Stdout)
        .init();
    zipp::run()
}
