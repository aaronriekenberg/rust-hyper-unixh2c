use std::collections::BTreeMap;

use tokio::sync::OnceCell;

pub type VersionInfoMap = BTreeMap<&'static str, &'static str>;

async fn build_version_info_map() -> VersionInfoMap {
    let mut map = VersionInfoMap::new();

    map.insert("build_timestamp", env!("VERGEN_BUILD_TIMESTAMP"));

    map.insert("cargo_pkg_version", env!("CARGO_PKG_VERSION"));

    map.insert("cargo_target_triple", env!("VERGEN_CARGO_TARGET_TRIPLE"));

    map.insert("rustc_channel", env!("VERGEN_RUSTC_CHANNEL"));
    map.insert("rustc_semver", env!("VERGEN_RUSTC_SEMVER"));

    map
}

pub async fn get_verison_info() -> &'static VersionInfoMap {
    static INSTANCE: OnceCell<VersionInfoMap> = OnceCell::const_new();

    INSTANCE.get_or_init(build_version_info_map).await
}
