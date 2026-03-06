use anyhow::Result;

pub const WINDOWS_SINGLETON_MUTEX_NAME: &str = "Global\\WorkDeskStudio.Singleton";

#[derive(Debug)]
pub enum InstanceAcquireResult {
    Primary(SingleInstanceGuard),
    Secondary,
}

pub fn acquire_single_instance() -> Result<InstanceAcquireResult> {
    acquire_single_instance_with_name(WINDOWS_SINGLETON_MUTEX_NAME)
}

pub fn acquire_single_instance_with_name(name: &str) -> Result<InstanceAcquireResult> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        let mut wide = name.encode_utf16().collect::<Vec<_>>();
        wide.push(0);

        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide.as_ptr()) };
        if handle.is_null() {
            return Err(anyhow::anyhow!("CreateMutexW failed for singleton mutex"));
        }

        let already_exists = unsafe { GetLastError() == ERROR_ALREADY_EXISTS };
        if already_exists {
            unsafe {
                CloseHandle(handle);
            }
            return Ok(InstanceAcquireResult::Secondary);
        }

        return Ok(InstanceAcquireResult::Primary(SingleInstanceGuard {
            inner: SingleInstanceGuardInner::Windows { handle },
        }));
    }

    #[cfg(not(windows))]
    {
        let lock_name = sanitize_lock_name(name);
        let lock_path = std::env::temp_dir().join(format!("workdesk_studio_{lock_name}.lock"));
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path);

        match file {
            Ok(file) => Ok(InstanceAcquireResult::Primary(SingleInstanceGuard {
                inner: SingleInstanceGuardInner::File { file, lock_path },
            })),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Ok(InstanceAcquireResult::Secondary)
            }
            Err(error) => {
                Err(error).with_context(|| format!("create singleton lock: {lock_path:?}"))
            }
        }
    }
}

#[derive(Debug)]
pub struct SingleInstanceGuard {
    inner: SingleInstanceGuardInner,
}

#[derive(Debug)]
enum SingleInstanceGuardInner {
    #[cfg(windows)]
    Windows {
        handle: windows_sys::Win32::Foundation::HANDLE,
    },
    #[cfg(not(windows))]
    File {
        file: std::fs::File,
        lock_path: std::path::PathBuf,
    },
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        match &self.inner {
            #[cfg(windows)]
            SingleInstanceGuardInner::Windows { handle } => unsafe {
                windows_sys::Win32::Foundation::CloseHandle(*handle);
            },
            #[cfg(not(windows))]
            SingleInstanceGuardInner::File { lock_path, file } => {
                let _keep_alive = file;
                let _ = std::fs::remove_file(lock_path);
            }
        }
    }
}

#[cfg(not(windows))]
fn sanitize_lock_name(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{acquire_single_instance_with_name, InstanceAcquireResult};
    use uuid::Uuid;

    #[test]
    fn acquires_primary_then_secondary() {
        let name = format!("Global\\WorkDeskStudio.Singleton.Test.{}", Uuid::new_v4());
        let primary = acquire_single_instance_with_name(&name).expect("acquire primary");
        let first_guard = match primary {
            InstanceAcquireResult::Primary(guard) => guard,
            InstanceAcquireResult::Secondary => panic!("expected primary"),
        };

        let secondary = acquire_single_instance_with_name(&name).expect("acquire secondary");
        assert!(matches!(secondary, InstanceAcquireResult::Secondary));
        drop(first_guard);

        let re_acquired = acquire_single_instance_with_name(&name).expect("re-acquire primary");
        assert!(matches!(re_acquired, InstanceAcquireResult::Primary(_)));
    }
}
