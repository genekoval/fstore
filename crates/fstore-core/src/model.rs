use serde::Serialize;

#[derive(Serialize, Debug, Clone, Copy)]
pub struct About {
    pub version: Version,
}

#[derive(Serialize, Debug, Clone, Copy)]
pub struct Version {
    pub number: &'static str,
    pub branch: &'static str,
    pub build_time: &'static str,
    pub build_os: &'static str,
    pub build_type: &'static str,
    pub commit_hash: &'static str,
    pub commit_date: &'static str,
    pub rust_version: &'static str,
    pub rust_channel: &'static str,
}
