//! Connection pool for WebDriver sessions
//!
//! Provides connection pooling with:
//! - Per-driver type pools
//! - Idle timeout for automatic cleanup
//! - Acquire/release semantics
//! - Health checking before returning connections

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use fantoccini::Client;
use tokio::sync::{Mutex, RwLock};

use crate::{config::Config, driver::DriverType, error::Result};

/// A pooled connection with metadata
#[derive(Debug)]
struct PooledConnection {
    /// The underlying WebDriver client
    client: Client,
    /// When this connection was last used
    last_used: Instant,
    /// Whether this connection is currently in use
    in_use: bool,
    /// The session ID for this connection
    session_id: String,
}

impl PooledConnection {
    fn new(client: Client, session_id: String) -> Self {
        Self {
            client,
            last_used: Instant::now(),
            in_use: false,
            session_id,
        }
    }

    fn mark_in_use(&mut self) {
        self.in_use = true;
        self.last_used = Instant::now();
    }

    fn mark_released(&mut self) {
        self.in_use = false;
        self.last_used = Instant::now();
    }

    fn is_idle_for(&self, duration: Duration) -> bool {
        !self.in_use && self.last_used.elapsed() > duration
    }
}

/// Statistics for a driver-specific pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub total_connections: usize,
    pub in_use: usize,
    pub idle: usize,
    pub total_acquisitions: u64,
    pub total_releases: u64,
    pub total_timeouts: u64,
    pub total_health_check_failures: u64,
}

/// Per-driver type connection pool
struct DriverPool {
    connections: Vec<PooledConnection>,
    max_connections: usize,
    stats: PoolStats,
}

impl DriverPool {
    fn new(max_connections: usize) -> Self {
        Self {
            connections: Vec::with_capacity(max_connections),
            max_connections,
            stats: PoolStats::default(),
        }
    }

    /// Try to acquire an idle connection from the pool
    fn try_acquire(&mut self) -> Option<(Client, String)> {
        for conn in &mut self.connections {
            if !conn.in_use {
                conn.mark_in_use();
                self.stats.in_use += 1;
                self.stats.idle = self.stats.idle.saturating_sub(1);
                self.stats.total_acquisitions += 1;
                return Some((conn.client.clone(), conn.session_id.clone()));
            }
        }
        None
    }

    /// Add a new connection to the pool
    fn add(&mut self, client: Client, session_id: String) -> bool {
        if self.connections.len() < self.max_connections {
            let mut conn = PooledConnection::new(client, session_id);
            conn.mark_in_use();
            self.connections.push(conn);
            self.stats.total_connections += 1;
            self.stats.in_use += 1;
            true
        } else {
            false
        }
    }

    /// Release a connection back to the pool
    fn release(&mut self, session_id: &str) -> bool {
        for conn in &mut self.connections {
            if conn.session_id == session_id && conn.in_use {
                conn.mark_released();
                self.stats.in_use = self.stats.in_use.saturating_sub(1);
                self.stats.idle += 1;
                self.stats.total_releases += 1;
                return true;
            }
        }
        false
    }

    /// Remove a connection from the pool
    fn remove(&mut self, session_id: &str) -> Option<Client> {
        if let Some(pos) = self.connections.iter().position(|c| c.session_id == session_id) {
            let conn = self.connections.remove(pos);
            self.stats.total_connections = self.stats.total_connections.saturating_sub(1);
            if conn.in_use {
                self.stats.in_use = self.stats.in_use.saturating_sub(1);
            } else {
                self.stats.idle = self.stats.idle.saturating_sub(1);
            }
            Some(conn.client)
        } else {
            None
        }
    }

    /// Remove all idle connections that have exceeded the timeout
    fn remove_idle(&mut self, idle_timeout: Duration) -> Vec<Client> {
        let mut removed = Vec::new();
        self.connections.retain(|conn| {
            if conn.is_idle_for(idle_timeout) {
                removed.push(conn.client.clone());
                self.stats.total_connections = self.stats.total_connections.saturating_sub(1);
                self.stats.idle = self.stats.idle.saturating_sub(1);
                false
            } else {
                true
            }
        });
        removed
    }

    /// Check if the pool can accept more connections
    fn has_capacity(&self) -> bool {
        self.connections.len() < self.max_connections
    }

    /// Get current stats
    fn get_stats(&self) -> PoolStats {
        self.stats.clone()
    }
}

/// Connection pool manager for all driver types
pub struct ConnectionPool {
    /// Per-driver type pools
    pools: Arc<RwLock<HashMap<DriverType, Mutex<DriverPool>>>>,
    /// Pool configuration
    config: PoolConfig,
    /// Whether the pool is enabled
    enabled: bool,
    /// Handle to the cleanup task (kept alive while pool exists)
    _cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Configuration for the connection pool
#[derive(Clone, Debug)]
pub struct PoolConfig {
    /// Maximum connections per driver type
    pub max_connections_per_driver: usize,
    /// Idle timeout before closing connections
    pub idle_timeout: Duration,
    /// Timeout for acquiring a connection
    pub acquire_timeout: Duration,
    /// Interval for running cleanup tasks
    pub cleanup_interval: Duration,
}

impl From<&Config> for PoolConfig {
    fn from(config: &Config) -> Self {
        Self {
            max_connections_per_driver: config.pool_max_connections_per_driver,
            idle_timeout: Duration::from_secs(config.pool_idle_timeout_secs),
            acquire_timeout: Duration::from_millis(config.pool_acquire_timeout_ms),
            cleanup_interval: Duration::from_secs(60), // Check every minute
        }
    }
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: &Config) -> Self {
        let pool_config = PoolConfig::from(config);
        let pools = Arc::new(RwLock::new(HashMap::new()));

        let cleanup_handle = if config.pool_enabled {
            Some(Self::start_cleanup_task(pools.clone(), pool_config.clone()))
        } else {
            None
        };

        Self {
            pools,
            config: pool_config,
            enabled: config.pool_enabled,
            _cleanup_handle: cleanup_handle,
        }
    }

    /// Start the background cleanup task
    fn start_cleanup_task(
        pools: Arc<RwLock<HashMap<DriverType, Mutex<DriverPool>>>>,
        config: PoolConfig,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.cleanup_interval);
            loop {
                interval.tick().await;
                Self::run_cleanup(&pools, config.idle_timeout).await;
            }
        })
    }

    /// Run cleanup on all pools
    async fn run_cleanup(
        pools: &Arc<RwLock<HashMap<DriverType, Mutex<DriverPool>>>>,
        idle_timeout: Duration,
    ) {
        let pools_guard = pools.read().await;
        for (driver_type, pool_mutex) in pools_guard.iter() {
            let mut pool = pool_mutex.lock().await;
            let removed = pool.remove_idle(idle_timeout);

            if !removed.is_empty() {
                tracing::debug!(
                    "Cleaned up {} idle {} connections",
                    removed.len(),
                    driver_type.browser_name()
                );

                // Close the removed clients
                for client in removed {
                    if let Err(e) = client.close().await {
                        tracing::warn!("Error closing idle connection: {}", e);
                    }
                }
            }
        }
    }

    /// Acquire a connection from the pool for a specific driver type
    /// Returns (session_id, client) if successful
    pub async fn acquire(
        &self,
        driver_type: &DriverType,
    ) -> Result<Option<(String, Client)>> {
        if !self.enabled {
            return Ok(None);
        }

        let pools = self.pools.read().await;
        if let Some(pool_mutex) = pools.get(driver_type) {
            let mut pool = pool_mutex.lock().await;

            // Try to acquire an existing idle connection
            if let Some((client, session_id)) = pool.try_acquire() {
                // Verify the connection is still healthy
                match tokio::time::timeout(
                    Duration::from_secs(2),
                    client.current_url(),
                ).await {
                    Ok(Ok(_)) => {
                        tracing::debug!(
                            "Acquired pooled {} connection: {}",
                            driver_type.browser_name(),
                            session_id
                        );
                        return Ok(Some((session_id, client)));
                    }
                    _ => {
                        // Connection is dead, remove it
                        tracing::debug!(
                            "Pooled connection {} is dead, removing",
                            session_id
                        );
                        pool.remove(&session_id);
                        pool.stats.total_health_check_failures += 1;
                    }
                }
            }
        }

        Ok(None)
    }

    /// Add a new connection to the pool
    /// Returns true if the connection was added, false if pool is full
    pub async fn add(
        &self,
        driver_type: DriverType,
        client: Client,
        session_id: String,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        let mut pools = self.pools.write().await;
        let pool = pools
            .entry(driver_type.clone())
            .or_insert_with(|| Mutex::new(DriverPool::new(self.config.max_connections_per_driver)));

        let mut pool_guard = pool.lock().await;
        let added = pool_guard.add(client, session_id.clone());

        if added {
            tracing::debug!(
                "Added {} connection to pool: {}",
                driver_type.browser_name(),
                session_id
            );
        } else {
            tracing::debug!(
                "Pool full for {}, connection {} not added",
                driver_type.browser_name(),
                session_id
            );
        }

        added
    }

    /// Release a connection back to the pool
    pub async fn release(&self, driver_type: &DriverType, session_id: &str) {
        if !self.enabled {
            return;
        }

        let pools = self.pools.read().await;
        if let Some(pool_mutex) = pools.get(driver_type) {
            let mut pool = pool_mutex.lock().await;
            if pool.release(session_id) {
                tracing::debug!(
                    "Released {} connection: {}",
                    driver_type.browser_name(),
                    session_id
                );
            }
        }
    }

    /// Remove a connection from the pool (e.g., on error)
    pub async fn remove(&self, driver_type: &DriverType, session_id: &str) -> Option<Client> {
        if !self.enabled {
            return None;
        }

        let pools = self.pools.read().await;
        if let Some(pool_mutex) = pools.get(driver_type) {
            let mut pool = pool_mutex.lock().await;
            if let Some(client) = pool.remove(session_id) {
                tracing::debug!(
                    "Removed {} connection from pool: {}",
                    driver_type.browser_name(),
                    session_id
                );
                return Some(client);
            }
        }
        None
    }

    /// Check if the pool has capacity for a driver type
    pub async fn has_capacity(&self, driver_type: &DriverType) -> bool {
        if !self.enabled {
            return true;
        }

        let pools = self.pools.read().await;
        if let Some(pool_mutex) = pools.get(driver_type) {
            let pool = pool_mutex.lock().await;
            pool.has_capacity()
        } else {
            true // No pool yet means we can create one
        }
    }

    /// Get statistics for all pools
    pub async fn get_stats(&self) -> HashMap<DriverType, PoolStats> {
        let pools = self.pools.read().await;
        let mut stats = HashMap::new();

        for (driver_type, pool_mutex) in pools.iter() {
            let pool = pool_mutex.lock().await;
            stats.insert(driver_type.clone(), pool.get_stats());
        }

        stats
    }

    /// Close all connections in all pools
    pub async fn close_all(&self) -> Result<()> {
        let pools = self.pools.read().await;

        for (driver_type, pool_mutex) in pools.iter() {
            let mut pool = pool_mutex.lock().await;

            // Collect all clients to close
            let clients: Vec<Client> = pool.connections.drain(..).map(|c| c.client).collect();

            tracing::debug!(
                "Closing {} {} connections from pool",
                clients.len(),
                driver_type.browser_name()
            );

            for client in clients {
                if let Err(e) = client.close().await {
                    tracing::warn!("Error closing pooled connection: {}", e);
                }
            }

            // Reset stats
            pool.stats = PoolStats::default();
        }

        Ok(())
    }

    /// Check if pooling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            webdriver_endpoint: "auto".to_string(),
            default_session_timeout_ms: 2000,
            auto_start_driver: true,
            preferred_driver: None,
            headless: true,
            concurrent_drivers: vec!["chrome".to_string()],
            driver_startup_timeout_ms: 10000,
            enable_performance_memory: false,
            pool_max_connections_per_driver: 3,
            pool_idle_timeout_secs: 300,
            pool_acquire_timeout_ms: 30000,
            pool_enabled: true,
        }
    }

    #[test]
    fn test_pool_config_from_config() {
        let config = create_test_config();
        let pool_config = PoolConfig::from(&config);

        assert_eq!(pool_config.max_connections_per_driver, 3);
        assert_eq!(pool_config.idle_timeout, Duration::from_secs(300));
        assert_eq!(pool_config.acquire_timeout, Duration::from_millis(30000));
    }

    #[test]
    fn test_driver_pool_capacity() {
        let pool = DriverPool::new(2);
        assert!(pool.has_capacity());
    }

    #[test]
    fn test_idle_time_check() {
        // Test the idle time calculation logic without creating a real PooledConnection
        let last_used = Instant::now() - Duration::from_secs(100);
        let idle_duration = last_used.elapsed();

        assert!(idle_duration > Duration::from_secs(50));
        assert!(idle_duration < Duration::from_secs(200));
    }
}
