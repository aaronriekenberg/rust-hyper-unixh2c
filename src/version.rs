use std::collections::BTreeMap;

pub type VersionInfoMap = BTreeMap<&'static str, &'static str>;

pub fn get_verison_info() -> VersionInfoMap {
    let mut map = VersionInfoMap::new();

    map.insert("CARGO_PKG_VERSION", env!("CARGO_PKG_VERSION"));

    map.insert("VERGEN_BUILD_TIMESTAMP", env!("VERGEN_BUILD_TIMESTAMP"));

    map.insert(
        "VERGEN_CARGO_TARGET_TRIPLE",
        env!("VERGEN_CARGO_TARGET_TRIPLE"),
    );

    map.insert("VERGEN_RUSTC_CHANNEL", env!("VERGEN_RUSTC_CHANNEL"));
    map.insert("VERGEN_RUSTC_SEMVER", env!("VERGEN_RUSTC_SEMVER"));

    map
}
