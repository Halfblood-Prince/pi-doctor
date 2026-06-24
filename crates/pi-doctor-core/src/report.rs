use crate::diagnosis::Finding;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub metadata: ReportMetadata,
    pub schema_version: &'static str,
    pub overall_status: OverallStatus,
    pub probe_health: Vec<ProbeHealth>,
    pub system: Option<SystemSummary>,
    pub config: Option<ConfigSummary>,
    pub camera: Option<CameraSummary>,
    pub python: Option<PythonSummary>,
    pub groups: Vec<FindingGroup>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportMetadata {
    pub command: String,
    pub pi_doctor_version: String,
    pub build_revision: Option<String>,
    pub target_architecture: String,
    pub supported_os: SupportedOs,
    pub probe_availability: ProbeAvailabilitySummary,
}

impl ReportMetadata {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            pi_doctor_version: env!("CARGO_PKG_VERSION").to_owned(),
            build_revision: option_env!("PI_DOCTOR_BUILD_REVISION")
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            target_architecture: std::env::consts::ARCH.to_owned(),
            supported_os: SupportedOs::default(),
            probe_availability: ProbeAvailabilitySummary::default(),
        }
    }

    pub fn with_supported_os(mut self, supported_os: SupportedOs) -> Self {
        self.supported_os = supported_os;
        self
    }

    pub fn with_probe_availability(mut self, probe_availability: ProbeAvailabilitySummary) -> Self {
        self.probe_availability = probe_availability;
        self
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SupportedOs {
    pub supported: bool,
    pub family: Option<String>,
    pub version: Option<String>,
    pub codename: Option<String>,
    pub reason: Option<String>,
}

impl Default for SupportedOs {
    fn default() -> Self {
        Self {
            supported: false,
            family: None,
            version: None,
            codename: None,
            reason: Some("support status was not evaluated".to_owned()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ProbeAvailabilitySummary {
    pub total: usize,
    pub success: usize,
    pub unavailable: usize,
    pub permission_denied: usize,
    pub command_failed: usize,
    pub parse_failed: usize,
    pub timed_out: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OverallStatus {
    Healthy,
    Warning,
    Degraded,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeHealth {
    pub name: &'static str,
    pub outcome: ProbeOutcome,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeOutcome {
    Success,
    Unavailable,
    PermissionDenied,
    CommandFailed,
    ParseFailed,
    TimedOut,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingGroup {
    pub domain: FindingDomain,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingDomain {
    System,
    Power,
    Thermal,
    Config,
    Gpio,
    Camera,
    Python,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SystemSummary {
    pub board_model: Option<String>,
    pub board_revision: Option<String>,
    pub architecture: Option<String>,
    pub distro_name: Option<String>,
    pub distro_version: Option<String>,
    pub distro_codename: Option<String>,
    pub kernel_release: Option<String>,
    pub is_raspberry_pi: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ConfigSummary {
    pub source_path: Option<String>,
    pub using_firmware_path: bool,
    pub legacy_path_present: bool,
    pub diagnostics_count: usize,
    pub entries: Vec<ConfigEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigEntry {
    pub line_number: usize,
    pub kind: ConfigEntryKind,
    pub raw: String,
    pub section: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigEntryKind {
    Blank,
    Comment,
    Section,
    Setting,
    Malformed,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CameraSummary {
    pub tool_used: Option<String>,
    pub rpicam_hello_present: bool,
    pub libcamera_hello_present: bool,
    pub video_devices: Vec<String>,
    pub cameras: Vec<CameraDevice>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CameraDevice {
    pub index: usize,
    pub name: String,
    pub mode_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PythonSummary {
    pub version: Option<String>,
    pub executable: Option<String>,
    pub in_virtualenv: bool,
    pub externally_managed: bool,
    pub detected_packages: Vec<String>,
}
