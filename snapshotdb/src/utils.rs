use std::io::Result;
use std::fs::File;
use std::sync::Arc;
use tokio::sync::Mutex;
#[cfg(target_family = "unix")]
use std::os::unix::prelude::FileExt;

pub async fn asyncify<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "background task failed",
        )),
    }
}

pub async fn custom_sync_range(file: Arc<File>, offset: u64, len: u64) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::io::AsRawFd;

        asyncify(move || unsafe {
            let ret =libc::sync_file_range(
                file.as_raw_fd(),
                i64::try_from(offset).unwrap(),
                i64::try_from(len).unwrap(),
                libc::SYNC_FILE_RANGE_WAIT_BEFORE
                    | libc::SYNC_FILE_RANGE_WRITE
                    | libc::SYNC_FILE_RANGE_WAIT_AFTER,
            );
            if ret < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }).await?;
    }

    #[cfg(not(target_os = "linux"))]
    asyncify(move || file.sync_all()).await?;

    Ok(())
}

pub fn mutex_vec_values<T: Clone>(vec: Vec<Mutex<T>>) -> Vec<T> {
    vec.into_iter().map(|mutex| mutex.into_inner()).collect()
}

pub fn to_mutex_vec<T:Clone>(items: &[T]) -> Vec<Mutex<T>> {
    items.iter().map(|item| Mutex::new(item.clone())).collect()
}

pub async fn read_exact_at(file: Arc<File>, offset: u64, len: usize) -> Result<Vec<u8>> {
    #[cfg(target_family = "unix")]
    {
        asyncify(move || {
            let mut buf = vec![0u8; len];
            file.read_exact_at(&mut buf, offset)?;
            Ok(buf)
        }).await
    }

    #[cfg(not(target_family = "unix"))]
    {
        unimplemented!("read_exact_at is only implemented for Unix systems")
    }
}

pub async fn write_all_at(file: Arc<File>, buf: &[u8], offset: u64) -> Result<()> {
    #[cfg(target_family = "unix")]
    {
        let buf = buf.to_vec();
        asyncify(move || {
            file.write_all_at(&buf, offset)?;
            Ok(())
        }).await
    }

    #[cfg(not(target_family = "unix"))]
    {
        unimplemented!("write_at is only implemented for Unix systems")
    }
}