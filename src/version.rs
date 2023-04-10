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

    map.insert("VERGEN_SYSINFO_NAME", env!("VERGEN_SYSINFO_NAME"));
    map.insert(
        "VERGEN_SYSINFO_OS_VERSION",
        env!("VERGEN_SYSINFO_OS_VERSION"),
    );
    map.insert(
        "VERGEN_SYSINFO_CPU_VENDOR",
        env!("VERGEN_SYSINFO_CPU_VENDOR"),
    );
    map.insert(
        "VERGEN_SYSINFO_CPU_CORE_COUNT",
        env!("VERGEN_SYSINFO_CPU_CORE_COUNT"),
    );
    map.insert("VERGEN_SYSINFO_CPU_NAME", env!("VERGEN_SYSINFO_CPU_NAME"));
    map.insert("VERGEN_SYSINFO_CPU_BRAND", env!("VERGEN_SYSINFO_CPU_BRAND"));
    map.insert(
        "VERGEN_SYSINFO_CPU_FREQUENCY",
        env!("VERGEN_SYSINFO_CPU_FREQUENCY"),
    );

    map
}
