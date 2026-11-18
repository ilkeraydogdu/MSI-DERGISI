use std::fs;
use std::path::Path;
use super::logs::pick_best_log_path;
use super::types::VhostInfo;

/// Discover all active domain vhosts under `/var/www/vhosts/` (including `.plesk.page` hosts).
/// Only skips Plesk system directories: `system`, `chroot`, `default`, `fs`, `fs-passwd`, `.skel`, and dotfiles.
pub fn discover_vhosts(base_path: &str, nginx_log_path: &str) -> Vec<VhostInfo> {
    let mut vhosts = Vec::new();
    let base_trimmed = base_path.trim_end_matches('/');
    // Plesk sets `/var/www/vhosts` to `0711` (`root:root`) so unprivileged service users
    // cannot list it; fall back to `/var/www/vhosts/system` (`0755`) when needed.
    let dir = match fs::read_dir(base_trimmed)
        .or_else(|_| fs::read_dir(format!("{}/system", base_trimmed)))
    {
        Ok(d) => d,
        Err(_) => return vhosts,
    };

    let nginx_dir = {
        let p = Path::new(nginx_log_path);
        if p.is_dir() {
            nginx_log_path.trim_end_matches('/').to_string()
        } else {
            p.parent()
                .and_then(|parent| parent.to_str())
                .unwrap_or("/var/log/nginx")
                .trim_end_matches('/')
                .to_string()
        }
    };

    for entry in dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip Plesk internal/system directories and wildcard subdomains (e.g. _.qordy.com)
        if name.starts_with('.')
            || name.starts_with("_.")
            || matches!(
                name.as_str(),
                "system" | "chroot" | "default" | "fs" | "fs-passwd" | ".skel"
            )
        {
            continue;
        }

        if !name.contains('.') {
            continue;
        }

        let domain = name.clone();
        let vhost_path = format!("{}/{}", base_trimmed, domain);

        let access_candidates = vec![
            format!("{}/logs/proxy_access_ssl_log", vhost_path),
            format!("{}/logs/access_ssl_log", vhost_path),
            format!("{}/logs/access_ssl_log.processed", vhost_path),
            format!("{}/logs/proxy_access_log", vhost_path),
            format!("{}/logs/access_log", vhost_path),
            format!("{}/logs/access_log.processed", vhost_path),
            format!("{}/{}.access.log", nginx_dir, domain),
            format!("/var/www/vhosts/system/{}/logs/proxy_access_ssl_log", domain),
        ];

        let error_candidates = vec![
            format!("{}/logs/proxy_error_log", vhost_path),
            format!("{}/logs/error_log", vhost_path),
            format!("{}/{}.error.log", nginx_dir, domain),
            format!("/var/www/vhosts/system/{}/logs/proxy_error_log", domain),
        ];

        vhosts.push(VhostInfo {
            domain,
            vhost_path,
            access_log_path: pick_best_log_path(&access_candidates),
            error_log_path: pick_best_log_path(&error_candidates),
        });
    }

    vhosts.sort_by(|a, b| a.domain.cmp(&b.domain));
    vhosts
}

