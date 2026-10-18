use std::sync::Mutex;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

struct State {
    active: usize,
    limit: usize,
}

pub struct DownloadSlots {
    state: Mutex<State>,
    changed: Notify,
}

pub struct Permit<'a>(&'a DownloadSlots);

impl DownloadSlots {
    pub fn new(limit: usize) -> Self {
        Self {
            state: Mutex::new(State { active: 0, limit }),
            changed: Notify::new(),
        }
    }

    pub fn set_limit(&self, limit: usize) -> Result<(), String> {
        if !(1..=8).contains(&limit) {
            return Err("同时下载数量必须在 1–8 之间".into());
        }
        self.state.lock().map_err(|e| e.to_string())?.limit = limit;
        self.changed.notify_waiters();
        Ok(())
    }

    pub async fn acquire(&self, cancel: &CancellationToken) -> Result<Permit<'_>, String> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if cancel.is_cancelled() {
                return Err("下载已取消".into());
            }
            {
                let mut state = self.state.lock().map_err(|e| e.to_string())?;
                if state.active < state.limit {
                    state.active += 1;
                    return Ok(Permit(self));
                }
            }
            tokio::select! {
                _ = cancel.cancelled() => return Err("下载已取消".into()),
                _ = changed => {}
            }
        }
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.state.lock() {
            state.active -= 1;
        }
        self.0.changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn changing_limit_preserves_running_tasks_and_blocks_new_ones_until_capacity_exists() {
        let slots = DownloadSlots::new(1);
        let cancel = CancellationToken::new();
        let first = slots.acquire(&cancel).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), slots.acquire(&cancel))
                .await
                .is_err()
        );
        slots.set_limit(3).unwrap();
        let second = slots.acquire(&cancel).await.unwrap();
        let third = slots.acquire(&cancel).await.unwrap();
        slots.set_limit(1).unwrap();
        drop(second);
        drop(third);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), slots.acquire(&cancel))
                .await
                .is_err()
        );
        drop(first);
        let _next = slots.acquire(&cancel).await.unwrap();
        cancel.cancel();
        assert!(slots.acquire(&cancel).await.is_err());
        assert!(slots.set_limit(0).is_err());
        assert!(slots.set_limit(9).is_err());
    }

    #[tokio::test]
    async fn increasing_limit_wakes_a_waiting_download() {
        let slots = DownloadSlots::new(1);
        let cancel = CancellationToken::new();
        let _running = slots.acquire(&cancel).await.unwrap();
        let waiting = slots.acquire(&cancel);
        tokio::pin!(waiting);
        tokio::select! {
            _ = &mut waiting => panic!("limit was exceeded"),
            _ = tokio::task::yield_now() => {}
        }
        slots.set_limit(2).unwrap();
        let _permit = tokio::time::timeout(Duration::from_secs(1), waiting)
            .await
            .unwrap()
            .unwrap();
    }
}
