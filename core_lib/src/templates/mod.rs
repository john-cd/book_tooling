pub mod tests;

/// Crate destinations.
pub enum DestinationKind {
    Category,
    Crate,
    Docs,
    GithubRepo,
    LibRs,
    CratesIo,
    Web,
}

impl DestinationKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Category => "cat",
            Self::Crate => "crate",
            Self::Docs => "docs",
            Self::GithubRepo => "repo",
            Self::LibRs => "lib.rs",
            Self::CratesIo => "crates.io",
            Self::Web => "website",
        }
    }

    pub fn prefix(&self) -> char {
        match self {
            Self::Category => 'c',
            Self::Crate => 'p',
            _ => 'c',
        }
    }
}

pub fn render_badge(kind: &DestinationKind, name: &str) -> String {
    let prefix = kind.prefix();
    let label = kind.label();
    format!("[![{name}][{prefix}~{name}~{label}~badge]][{prefix}~{name}~{label}]{{{{hi:{name}}}}}")
}

pub fn render_link(kind: &DestinationKind, name: &str) -> String {
    match kind {
        DestinationKind::Category => format!("[{name}][cat~{name}]↗{{{{hi:{name}}}}}"),
        _ => {
            let prefix = kind.prefix();
            let label = kind.label();
            format!("[{name}][{prefix}~{name}~{label}]↗{{{{hi:{name}}}}}")
        }
    }
}
