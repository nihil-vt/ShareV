//! Optional feature dependencies.
//!
//! Some features (OCR, and anything added later) depend on executables that are
//! not bundled with ShareV. Each such feature can be switched off in the config
//! (`[features]`); a disabled feature never requires its dependency to be
//! installed. When a feature is enabled but its binary is missing, this module
//! knows the right install command for the host distribution so the UI and CLI
//! can prompt the user to install it.

use crate::capture::which;

/// A toggleable feature with an optional system dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Ocr,
}

impl Feature {
    pub fn id(self) -> &'static str {
        match self {
            Feature::Ocr => "ocr",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Feature::Ocr => "OCR (Tesseract)",
        }
    }

    /// The executable the feature runs on the host.
    pub fn binary(self) -> &'static str {
        match self {
            Feature::Ocr => "tesseract",
        }
    }

    /// Install commands keyed by package-manager id. Packagers listed here
    /// cover the common GNU/Linux desktop distributions.
    pub fn install_commands(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Feature::Ocr => &[
                ("apt", "sudo apt install tesseract-ocr tesseract-ocr-eng"),
                ("dnf", "sudo dnf install tesseract tesseract-langpack-eng"),
                ("pacman", "sudo pacman -S tesseract tesseract-data-eng"),
                (
                    "zypper",
                    "sudo zypper install tesseract-ocr tesseract-ocr-traineddata-english",
                ),
                ("apk", "sudo apk add tesseract-ocr tesseract-ocr-data-eng"),
                ("xbps", "sudo xbps-install tesseract-ocr"),
                ("nix", "nix profile install nixpkgs#tesseract"),
            ],
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "ocr" => Some(Feature::Ocr),
            _ => None,
        }
    }
}

/// The best install command for `feature` on the current distribution, or
/// `None` when the distribution can't be mapped to a known package manager.
pub fn install_command(feature: Feature) -> Option<String> {
    let pm = package_manager_id()?;
    feature
        .install_commands()
        .iter()
        .find(|(id, _)| *id == pm)
        .map(|(_, cmd)| (*cmd).to_string())
}

/// Whether the feature's binary is on `PATH`.
pub fn is_installed(feature: Feature) -> bool {
    which(feature.binary()).is_some()
}

/// The package-manager id for the host distribution, resolved from
/// `/etc/os-release` (`ID` first, then the `ID_LIKE` chain).
pub fn package_manager_id() -> Option<&'static str> {
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut id: Option<&str> = None;
    let mut like: Vec<&str> = Vec::new();
    for line in os_release.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
        match key {
            "ID" => id = Some(value),
            "ID_LIKE" => like.extend(value.split_whitespace()),
            _ => {}
        }
    }
    for candidate in id.into_iter().chain(like) {
        if let Some(pm) = map_distro(candidate) {
            return Some(pm);
        }
    }
    None
}

fn map_distro(id: &str) -> Option<&'static str> {
    Some(match id {
        "ubuntu" | "debian" | "linuxmint" | "pop" | "elementary" | "kali" | "raspbian" => "apt",
        "fedora" | "rhel" | "centos" | "rocky" | "almalinux" | "oracle" | "amzn" => "dnf",
        "arch" | "manjaro" | "endeavouros" | "cachyos" | "garuda" | "artix" => "pacman",
        "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" | "sles" | "sled" => "zypper",
        "alpine" => "apk",
        "void" => "xbps",
        "nixos" => "nix",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_features() {
        assert_eq!(Feature::parse("ocr"), Some(Feature::Ocr));
        assert_eq!(Feature::parse("nope"), None);
    }

    #[test]
    fn maps_distro_families_to_package_managers() {
        assert_eq!(map_distro("ubuntu"), Some("apt"));
        assert_eq!(map_distro("fedora"), Some("dnf"));
        assert_eq!(map_distro("arch"), Some("pacman"));
        assert_eq!(map_distro("opensuse-tumbleweed"), Some("zypper"));
        assert_eq!(map_distro("alpine"), Some("apk"));
        assert_eq!(map_distro("void"), Some("xbps"));
        assert_eq!(map_distro("windows"), None);
    }

    #[test]
    fn install_command_for_a_known_package_manager() {
        let commands = Feature::Ocr.install_commands();
        let apt = commands.iter().find(|(id, _)| *id == "apt");
        let (_, cmd) = apt.expect("apt command exists");
        assert!(cmd.contains("tesseract"));
    }
}