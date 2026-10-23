//! Every sentence the backend shows to people, in one place.
//!
//! Rules: say what happened and what to do next, in one or two plain sentences; never mention
//! "rcd", "RC" or internals. The raw technical text goes into `AppError::details`, which the UI
//! shows behind "Show details". The one exception to "everything here" is the wording of the
//! add-storage wizards (titles, field labels, help), which lives with the form definitions in
//! `remotes::providers`.

use crate::error::{AppError, ErrorKind};

// Storage engine (rclone) lifecycle.
pub const ENGINE_NOT_READY: &str =
    "Orbit is still starting its storage engine. Try again in a moment.";
pub const ENGINE_UNREACHABLE: &str = "Orbit lost contact with its storage engine. It restarts on its own; if this keeps happening, restart it from Settings.";
pub const ENGINE_BAD_ANSWER: &str =
    "Orbit got an answer it could not read from its storage engine. Try again.";
pub const ENGINE_REQUEST_FAILED: &str =
    "Orbit could not talk to its storage engine. Try again, or restart it from Settings.";
pub const ENGINE_EXITED_WHILE_STARTING: &str = "The storage engine stopped while starting. Check the custom engine path in Settings, or reinstall Orbit.";
pub const ENGINE_SLOW_START: &str = "The storage engine did not answer in time. Restart it from Settings; if that fails, restart Orbit.";
pub const ENGINE_EXITED: &str = "The storage engine stopped unexpectedly. Orbit is restarting it.";
pub const ENGINE_UNRESPONSIVE: &str =
    "The storage engine stopped answering. Orbit is restarting it.";
pub const ENGINE_NOT_FOUND: &str = "Orbit could not find its storage engine. Reinstall Orbit, or pick the program by hand under Settings › Advanced.";
pub const ENGINE_CUSTOM_PATH_MISSING: &str = "The custom engine program chosen in Settings is not there any more. Pick it again, or switch back to the built-in one.";
pub const ENGINE_COULD_NOT_START: &str = "Orbit could not start its storage engine. Reinstall Orbit, or check the custom engine path in Settings.";

// Quitting.
pub const QUIT_TITLE: &str = "Quit Orbit?";
pub const QUIT_ANYWAY: &str = "Quit";
pub const KEEP_RUNNING: &str = "Keep running";
pub fn quit_busy(count: usize) -> String {
    if count == 1 {
        "A transfer or backup is still running. Quitting stops it; it can be run again later."
            .to_owned()
    } else {
        format!(
            "{count} transfers or backups are still running. Quitting stops them; they can be run again later."
        )
    }
}

// Settings.
pub const CONCURRENCY_RANGE: &str = "Files at the same time must be between 1 and 8.";
pub const AUTOSTART_FAILED: &str = "Orbit could not change the start-at-sign-in setting on this computer. The details may say why.";

// Cloud storage (remotes) and the wizards.
pub const UNKNOWN_SERVICE: &str = "Orbit does not know that kind of storage.";
pub const SERVICE_USES_BROWSER: &str = "This service signs in through your web browser.";
pub const SERVICE_USES_CREDENTIALS: &str =
    "This service uses a username and password or keys, not a browser sign-in.";
pub const NAME_BLANK: &str = "Give this storage a name.";
pub const NAME_RULES: &str =
    "Names can use letters, numbers, dashes and underscores, up to 40 characters.";
pub const SIGN_IN_FAILED: &str =
    "The sign-in did not finish. Try again, and make sure you complete the steps in the browser.";
pub const CANNOT_EDIT_KIND: &str =
    "This storage was set up outside Orbit, so it cannot be edited here.";
pub const TEST_OK: &str = "Connected.";
pub const PASSWORD_OR_KEY_FILE: &str = "Enter a password or choose a key file.";
pub const ENDPOINT_REQUIRED: &str = "Enter the endpoint address for this service.";

pub fn name_taken(name: &str) -> String {
    format!("You already have storage called {name}. Pick another name.")
}
pub fn remote_missing(name: &str) -> String {
    format!("There is no storage called {name} any more. Refresh the list.")
}
pub fn field_required(label: &str) -> String {
    format!("Please fill in {}.", label.to_lowercase())
}
pub fn field_not_a_number(label: &str) -> String {
    format!("{label} must be a number.")
}
pub fn field_out_of_range(label: &str, min: u16, max: u16) -> String {
    format!("{label} must be between {min} and {max}.")
}
pub fn field_bad_choice(label: &str) -> String {
    format!("Choose a valid option for {}.", label.to_lowercase())
}

// Files and transfers.
pub const TRANSFER_ONE_ITEM: &str = "Choose a single file or folder to transfer.";
pub const LOCAL_TO_REMOTE_IS_UPLOAD: &str = "Files on this computer are uploaded, not moved. Delete the local copy yourself afterwards if you want.";
pub const REMOTE_TO_LOCAL_IS_DOWNLOAD: &str = "Downloads copy files. Delete the copy in the cloud yourself afterwards if you want to move it.";
pub const LOCAL_TO_LOCAL_UNSUPPORTED: &str = "Orbit copies between this computer and your cloud storage, not between two places on this computer.";
pub const TRANSFER_GONE: &str = "That transfer is no longer in the list.";
pub const TRANSFER_FAILED: &str =
    "The transfer did not finish. Check the connection and try again.";
pub fn local_path_unreadable(path: &str) -> String {
    format!("Orbit could not read {path}. Check that it still exists and that you can open it.")
}
pub fn local_path_no_name(path: &str) -> String {
    format!("{path} has no file name, so it cannot be transferred.")
}
pub fn local_path_no_parent(path: &str) -> String {
    format!("{path} has no folder above it, so it cannot be transferred.")
}

// Backup & sync jobs.
pub const JOB_NAME_BLANK: &str = "Give this backup a name.";
pub const JOB_NAME_TOO_LONG: &str = "Backup names can be up to 60 characters.";
pub const JOB_NEEDS_CLOUD_SIDE: &str = "A backup needs cloud storage on one side. Orbit does not copy between two folders on this computer.";
pub const JOB_SAME_FOLDERS: &str = "The source and destination must be different folders.";
pub const JOB_INTERVAL_TOO_SHORT: &str = "Run at least one minute apart.";
pub const JOB_INTERVAL_TOO_LONG: &str =
    "Runs can be up to seven days apart. For longer gaps, run the backup by hand.";
pub const JOB_BAD_TIME: &str = "Choose a time of day between 00:00 and 23:59.";
pub const JOB_RUNNING_EDIT: &str = "Stop this backup before changing it.";
pub const JOB_RUNNING_DELETE: &str = "Stop this backup before deleting it.";
pub const JOB_GONE: &str = "That backup no longer exists.";
pub const JOB_ALREADY_RUNNING: &str = "This backup is already running.";
pub const JOB_NOT_RUNNING: &str = "This backup is not running.";
pub const RUN_GONE: &str = "That run is no longer in the history.";
pub const RUN_INTERRUPTED: &str = "Orbit closed before this run finished.";
pub const RUN_PROBLEM: &str = "The run finished with a problem. Open the log for details.";
pub const LOG_UNREADABLE: &str = "Orbit could not open that log file.";
pub const LOG_TRUNCATED: &str = "…earlier lines are in the log file…";
pub const LOG_FINISHED_OK: &str = "Finished. Everything was transferred.";
pub const LOG_STOPPED: &str = "Stopped by you.";
pub fn log_finished_with_problem(message: &str) -> String {
    format!("Finished with a problem: {message}")
}

// Drives and folders (mounts).
pub const MOUNT_NEEDS_FUSE: &str =
    "To show storage as a folder, install FUSE 3 first. On Ubuntu: sudo apt install fuse3.";
pub const MOUNT_NEEDS_WINFSP: &str =
    "To show storage as a drive, install WinFsp first. It is free and takes a minute.";
pub const MOUNT_UNSUPPORTED: &str =
    "Showing storage as a drive or folder is not available on this computer.";
pub const MOUNT_NO_LETTER: &str = "Every drive letter is taken. Remove another drive first.";
pub const MOUNT_NO_HOME: &str = "Orbit could not find your home folder.";
pub const MOUNT_WHERE: &str = "Choose where the storage should appear.";

/// Explains an error text from the storage engine in plain words. The raw text goes to details.
pub fn explain_engine_error(raw: &str) -> &'static str {
    let lower = raw.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
    if has(&["no such host", "lookup "]) {
        "Orbit could not find that server. Check the address and your network connection."
    } else if has(&["connection refused"]) {
        "The server refused the connection. Check the port, and that the service is running."
    } else if has(&["timeout", "deadline exceeded", "timed out"]) {
        "The server did not answer in time. Check your network connection and the address."
    } else if has(&["x509", "certificate"]) {
        "The server's security certificate could not be verified, so Orbit did not connect."
    } else if has(&[
        "unauthorized",
        "401",
        "403",
        "accessdenied",
        "invalidaccesskeyid",
        "signaturedoesnotmatch",
        "permission denied",
        "unable to authenticate",
        "no supported methods remain",
        "invalid_grant",
        "logon failure",
        "bad password",
    ]) {
        "The sign-in details were not accepted. Check the username, password or keys."
    } else if has(&["quota", "insufficient", "no space", "storage full"]) {
        "There is no room left on that storage."
    } else if has(&[
        "directory not found",
        "object not found",
        "not found",
        "no such file",
    ]) {
        "That file or folder is no longer there. Refresh and try again."
    } else if has(&["already exists", "file exists"]) {
        "Something with that name already exists there."
    } else if has(&["is a directory", "not a directory"]) {
        "Orbit expected a file but found a folder, or the other way round."
    } else {
        "The storage service reported a problem. The details may say more."
    }
}

/// Wraps a raw engine error: plain message, raw text in details.
pub fn engine_error(raw: &str) -> AppError {
    AppError::new(ErrorKind::Rclone, explain_engine_error(raw)).with_details(raw)
}

/// Default message for a failed connection check, with the raw text in details.
pub fn connection_failed(raw: &str) -> &'static str {
    let message = explain_engine_error(raw);
    if message.starts_with("The storage service reported") {
        "Orbit could not reach this storage. Check the details, then the address and sign-in."
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_errors_get_plain_explanations() {
        assert!(
            explain_engine_error("dial tcp: lookup nope.example: no such host")
                .contains("find that server")
        );
        assert!(explain_engine_error("connect: connection refused").contains("refused"));
        assert!(explain_engine_error("context deadline exceeded").contains("in time"));
        assert!(explain_engine_error("ssh: unable to authenticate").contains("sign-in details"));
        assert!(explain_engine_error("SignatureDoesNotMatch").contains("sign-in details"));
        assert!(explain_engine_error("directory not found").contains("no longer there"));
        assert!(explain_engine_error("something odd").contains("reported a problem"));
    }

    #[test]
    fn wrapped_errors_keep_the_raw_text_as_details() {
        let err = engine_error("directory not found");
        assert_eq!(err.details.as_deref(), Some("directory not found"));
        assert!(!err.message.contains("directory not found"));
    }

    #[test]
    fn no_message_mentions_internals() {
        for text in [
            ENGINE_NOT_READY,
            ENGINE_UNREACHABLE,
            ENGINE_BAD_ANSWER,
            ENGINE_REQUEST_FAILED,
            ENGINE_EXITED_WHILE_STARTING,
            ENGINE_SLOW_START,
            ENGINE_EXITED,
            ENGINE_UNRESPONSIVE,
            ENGINE_NOT_FOUND,
            ENGINE_CUSTOM_PATH_MISSING,
            ENGINE_COULD_NOT_START,
            UNKNOWN_SERVICE,
            SERVICE_USES_BROWSER,
            SERVICE_USES_CREDENTIALS,
            SIGN_IN_FAILED,
            CANNOT_EDIT_KIND,
            TRANSFER_FAILED,
            JOB_NEEDS_CLOUD_SIDE,
            MOUNT_NEEDS_FUSE,
            MOUNT_NEEDS_WINFSP,
        ] {
            let lower = text.to_lowercase();
            assert!(
                !lower.contains("rcd") && !lower.contains(" rc ") && !lower.contains("rclone"),
                "{text}"
            );
        }
    }
}
