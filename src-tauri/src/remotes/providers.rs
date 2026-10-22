//! Provider definitions for the add-remote wizards, and turning form values into rclone parameters.

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::BTreeMap;

/// How the wizard signs the user in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthKind {
    /// rclone's own sign-in in the browser (OAuth).
    Browser,
    /// Keys, passwords or a key file typed into the wizard.
    Credentials,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Choice {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FieldKind {
    Text,
    /// Never shown again after it is saved.
    Secret,
    Number {
        min: u16,
        max: u16,
    },
    Choice {
        options: &'static [Choice],
    },
}

/// One input on a wizard step. `key` is also the rclone parameter name.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub key: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub kind: FieldKind,
    pub required: bool,
    pub default: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDef {
    pub id: &'static str,
    pub title: &'static str,
    pub rclone_type: &'static str,
    pub tagline: &'static str,
    pub auth: AuthKind,
    pub fields: &'static [FieldDef],
}

const DRIVE_SCOPES: &[Choice] = &[
    Choice {
        value: "drive",
        label: "Full access to my files",
    },
    Choice {
        value: "drive.readonly",
        label: "Read-only access",
    },
];

const S3_PROVIDERS: &[Choice] = &[
    Choice {
        value: "AWS",
        label: "Amazon S3",
    },
    Choice {
        value: "Minio",
        label: "MinIO",
    },
    Choice {
        value: "Wasabi",
        label: "Wasabi",
    },
    Choice {
        value: "Cloudflare",
        label: "Cloudflare R2",
    },
    Choice {
        value: "Other",
        label: "Backblaze B2 or another S3-compatible service",
    },
];

const DRIVE_FIELDS: &[FieldDef] = &[FieldDef {
    key: "scope",
    label: "Access",
    help: "Read-only stops Orbit from changing anything in this account.",
    kind: FieldKind::Choice {
        options: DRIVE_SCOPES,
    },
    required: false,
    default: Some("drive"),
}];

const S3_FIELDS: &[FieldDef] = &[
    FieldDef {
        key: "provider",
        label: "Service",
        help: "Pick the service this storage belongs to.",
        kind: FieldKind::Choice {
            options: S3_PROVIDERS,
        },
        required: true,
        default: Some("AWS"),
    },
    FieldDef {
        key: "access_key_id",
        label: "Access key ID",
        help: "Shown in your storage provider's key settings.",
        kind: FieldKind::Text,
        required: true,
        default: None,
    },
    FieldDef {
        key: "secret_access_key",
        label: "Secret access key",
        help: "Stored by rclone and never shown again.",
        kind: FieldKind::Secret,
        required: true,
        default: None,
    },
    FieldDef {
        key: "endpoint",
        label: "Endpoint",
        help: "Leave empty for Amazon S3. Other services give you an address such as s3.example.com.",
        kind: FieldKind::Text,
        required: false,
        default: None,
    },
    FieldDef {
        key: "region",
        label: "Region",
        help: "Optional. Leave empty unless your provider asks for one.",
        kind: FieldKind::Text,
        required: false,
        default: None,
    },
];

const SFTP_FIELDS: &[FieldDef] = &[
    FieldDef {
        key: "host",
        label: "Server address",
        help: "A host name or IP address, such as files.example.com.",
        kind: FieldKind::Text,
        required: true,
        default: None,
    },
    FieldDef {
        key: "port",
        label: "Port",
        help: "Usually 22.",
        kind: FieldKind::Number { min: 1, max: 65535 },
        required: true,
        default: Some("22"),
    },
    FieldDef {
        key: "user",
        label: "Username",
        help: "The account to sign in with on the server.",
        kind: FieldKind::Text,
        required: true,
        default: None,
    },
    FieldDef {
        key: "pass",
        label: "Password",
        help: "Leave empty if you sign in with a key file.",
        kind: FieldKind::Secret,
        required: false,
        default: None,
    },
    FieldDef {
        key: "key_file",
        label: "Key file",
        help: "Path to a private key, such as ~/.ssh/id_ed25519. Leave empty if you use a password.",
        kind: FieldKind::Text,
        required: false,
        default: None,
    },
];

pub const PROVIDERS: &[ProviderDef] = &[
    ProviderDef {
        id: "drive",
        title: "Google Drive",
        rclone_type: "drive",
        tagline: "Sign in with your Google account.",
        auth: AuthKind::Browser,
        fields: DRIVE_FIELDS,
    },
    ProviderDef {
        id: "onedrive",
        title: "OneDrive",
        rclone_type: "onedrive",
        tagline: "Sign in with your Microsoft account.",
        auth: AuthKind::Browser,
        fields: &[],
    },
    ProviderDef {
        id: "dropbox",
        title: "Dropbox",
        rclone_type: "dropbox",
        tagline: "Sign in with your Dropbox account.",
        auth: AuthKind::Browser,
        fields: &[],
    },
    ProviderDef {
        id: "s3",
        title: "S3-compatible storage",
        rclone_type: "s3",
        tagline: "Amazon S3, MinIO, Wasabi, Cloudflare R2, Backblaze B2 and similar.",
        auth: AuthKind::Credentials,
        fields: S3_FIELDS,
    },
    ProviderDef {
        id: "sftp",
        title: "SFTP server",
        rclone_type: "sftp",
        tagline: "A server you reach over SSH, with a password or key file.",
        auth: AuthKind::Credentials,
        fields: SFTP_FIELDS,
    },
];

pub fn all() -> &'static [ProviderDef] {
    PROVIDERS
}

pub fn find(id: &str) -> Option<&'static ProviderDef> {
    PROVIDERS.iter().find(|provider| provider.id == id)
}

/// Finds the wizard provider that owns an rclone remote type, if there is one.
pub fn for_rclone_type(rclone_type: &str) -> Option<&'static ProviderDef> {
    PROVIDERS
        .iter()
        .find(|provider| provider.rclone_type == rclone_type)
}

/// Names of fields that hold secrets for a provider. Their values never go back to the UI.
pub fn secret_keys(provider: &ProviderDef) -> impl Iterator<Item = &'static str> + '_ {
    provider
        .fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Secret))
        .map(|field| field.key)
}

/// Checks the values from a wizard form and returns the rclone parameters to save.
///
/// `update` is `None` when creating a remote. Then an empty optional field is left out.
/// When editing, `Some(saved_secrets)` lists the secrets already stored: a blank secret keeps its
/// stored value, and a blank optional field is sent as "" so rclone clears it.
pub fn build_parameters(
    provider: &ProviderDef,
    values: &BTreeMap<String, String>,
    update: Option<&[String]>,
) -> AppResult<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();
    for field in provider.fields {
        let raw = values
            .get(field.key)
            .map(|value| value.trim())
            .unwrap_or("");
        if raw.is_empty() {
            let kept = update.is_some_and(|saved| saved.iter().any(|key| key == field.key));
            if kept {
                continue;
            }
            if let Some(default) = field.default {
                params.insert(field.key.to_owned(), default.to_owned());
            } else if field.required {
                return Err(AppError::invalid(format!(
                    "Please fill in {}.",
                    field.label.to_lowercase()
                )));
            } else if update.is_some() {
                params.insert(field.key.to_owned(), String::new());
            }
            continue;
        }
        match field.kind {
            FieldKind::Number { min, max } => {
                let number: u32 = raw
                    .parse()
                    .map_err(|_| AppError::invalid(format!("{} must be a number.", field.label)))?;
                if number < u32::from(min) || number > u32::from(max) {
                    return Err(AppError::invalid(format!(
                        "{} must be between {min} and {max}.",
                        field.label
                    )));
                }
            }
            FieldKind::Choice { options } => {
                if !options.iter().any(|option| option.value == raw) {
                    return Err(AppError::invalid(format!(
                        "Choose a valid option for {}.",
                        field.label.to_lowercase()
                    )));
                }
            }
            FieldKind::Text | FieldKind::Secret => {}
        }
        params.insert(field.key.to_owned(), raw.to_owned());
    }
    check_cross_field_rules(provider, &mut params, update)?;
    Ok(params)
}

fn check_cross_field_rules(
    provider: &ProviderDef,
    params: &mut BTreeMap<String, String>,
    update: Option<&[String]>,
) -> AppResult<()> {
    let filled = |params: &BTreeMap<String, String>, key: &str| {
        params.get(key).is_some_and(|value| !value.is_empty())
    };
    match provider.id {
        "sftp" => {
            let kept_password = update.is_some_and(|saved| saved.iter().any(|key| key == "pass"));
            let has_password =
                filled(params, "pass") || (kept_password && !params.contains_key("pass"));
            if !has_password && !filled(params, "key_file") {
                return Err(AppError::invalid("Enter a password or choose a key file."));
            }
        }
        "s3" => {
            let service = params.get("provider").map(String::as_str).unwrap_or("AWS");
            if service != "AWS" && !filled(params, "endpoint") {
                return Err(AppError::invalid(
                    "Enter the endpoint address for this service.",
                ));
            }
            if service == "AWS" && update.is_none() {
                // Amazon S3 needs no endpoint, so an empty one is not stored when creating.
                params.remove("endpoint");
            }
        }
        _ => {}
    }
    Ok(())
}

/// Validates a remote name: 1 to 40 letters, digits, dashes or underscores.
pub fn validate_remote_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim();
    let valid_chars = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if trimmed.is_empty() {
        return Err(AppError::invalid("Give this connection a name."));
    }
    if trimmed.len() > 40 || !valid_chars {
        return Err(AppError::invalid(
            "Names can use letters, numbers, dashes and underscores, up to 40 characters.",
        ));
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn every_provider_has_a_unique_id_and_type() {
        let mut ids: Vec<_> = PROVIDERS.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), PROVIDERS.len());
        assert!(PROVIDERS.iter().all(|p| find(p.id).is_some()));
    }

    #[test]
    fn browser_providers_have_no_typed_secrets() {
        for provider in PROVIDERS.iter().filter(|p| p.auth == AuthKind::Browser) {
            assert_eq!(secret_keys(provider).count(), 0, "{}", provider.id);
        }
    }

    #[test]
    fn s3_requires_keys() {
        let s3 = find("s3").unwrap();
        let err = build_parameters(
            s3,
            &values(&[("provider", "AWS"), ("access_key_id", "AK")]),
            None,
        )
        .unwrap_err();
        assert_eq!(err.message, "Please fill in secret access key.");
    }

    #[test]
    fn s3_builds_parameters_and_drops_empty_endpoint() {
        let s3 = find("s3").unwrap();
        let params = build_parameters(
            s3,
            &values(&[
                ("provider", "AWS"),
                ("access_key_id", " AK "),
                ("secret_access_key", "SK"),
                ("endpoint", "  "),
            ]),
            None,
        )
        .unwrap();
        assert_eq!(params["access_key_id"], "AK");
        assert_eq!(params["secret_access_key"], "SK");
        assert!(!params.contains_key("endpoint"));
    }

    #[test]
    fn s3_compatible_service_needs_endpoint() {
        let s3 = find("s3").unwrap();
        let err = build_parameters(
            s3,
            &values(&[
                ("provider", "Cloudflare"),
                ("access_key_id", "AK"),
                ("secret_access_key", "SK"),
            ]),
            None,
        )
        .unwrap_err();
        assert!(err.message.contains("endpoint"));
    }

    #[test]
    fn sftp_needs_password_or_key_file() {
        let sftp = find("sftp").unwrap();
        let base = [("host", "files.example.com"), ("user", "ana")];
        let err = build_parameters(sftp, &values(&base), None).unwrap_err();
        assert_eq!(err.message, "Enter a password or choose a key file.");

        let mut with_key = base.to_vec();
        with_key.push(("key_file", "~/.ssh/id_ed25519"));
        let params = build_parameters(sftp, &values(&with_key), None).unwrap();
        assert_eq!(params["port"], "22");
    }

    #[test]
    fn sftp_port_is_checked() {
        let sftp = find("sftp").unwrap();
        let err = build_parameters(
            sftp,
            &values(&[
                ("host", "h"),
                ("user", "u"),
                ("pass", "p"),
                ("port", "99999"),
            ]),
            None,
        )
        .unwrap_err();
        assert!(err.message.contains("between 1 and 65535"));
        let err = build_parameters(
            sftp,
            &values(&[("host", "h"), ("user", "u"), ("pass", "p"), ("port", "ssh")]),
            None,
        )
        .unwrap_err();
        assert!(err.message.contains("must be a number"));
    }

    #[test]
    fn drive_scope_defaults_and_rejects_unknown() {
        let drive = find("drive").unwrap();
        let params = build_parameters(drive, &BTreeMap::new(), None).unwrap();
        assert_eq!(params["scope"], "drive");
        let err = build_parameters(drive, &values(&[("scope", "everything")]), None).unwrap_err();
        assert!(err.message.contains("valid option"));
    }

    #[test]
    fn editing_keeps_saved_secrets_and_clears_blank_optionals() {
        let sftp = find("sftp").unwrap();
        let saved = vec!["pass".to_owned()];
        let params = build_parameters(
            sftp,
            &values(&[
                ("host", "new.example.com"),
                ("user", "ana"),
                ("pass", ""),
                ("key_file", ""),
            ]),
            Some(&saved),
        )
        .unwrap();
        assert_eq!(params["host"], "new.example.com");
        assert!(
            !params.contains_key("pass"),
            "a blank saved secret must not be overwritten"
        );
        assert_eq!(
            params["key_file"], "",
            "a cleared optional field is sent as empty"
        );
    }

    #[test]
    fn editing_requires_a_password_or_key_when_none_is_saved() {
        let sftp = find("sftp").unwrap();
        let err = build_parameters(
            sftp,
            &values(&[("host", "h"), ("user", "u"), ("pass", "")]),
            Some(&[]),
        )
        .unwrap_err();
        assert_eq!(err.message, "Enter a password or choose a key file.");
    }

    #[test]
    fn remote_names() {
        assert_eq!(
            validate_remote_name("  Work-Drive_2 ").unwrap(),
            "Work-Drive_2"
        );
        assert!(validate_remote_name("").is_err());
        assert!(validate_remote_name("has space").is_err());
        assert!(validate_remote_name("colon:name").is_err());
        assert!(validate_remote_name(&"a".repeat(41)).is_err());
        assert!(validate_remote_name(&"a".repeat(40)).is_ok());
    }
}
