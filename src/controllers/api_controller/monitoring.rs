use super::*;

// ============================================================================

pub async fn admin_monitoring_bundle(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let needs_system_seed = cache.read().await.system.is_none();
            if needs_system_seed {
                monitoring_service::tick_system_stats(cache).await;
            }
            let mut bundle = monitoring_service::collect_full_bundle(cache).await;
            if bundle.attacks.is_empty() {
                bundle.attacks = state.db.get_recent_attack_events(50);
            }
            ApiResponse::ok(json!(bundle)).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_system(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let needs_system_seed = cache.read().await.system.is_none();
            if needs_system_seed {
                monitoring_service::tick_system_stats(cache).await;
            }
            let c = cache.read().await;
            ApiResponse::ok(json!({ "system": c.system })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_domains(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let domains: Vec<_> = c.domains.values().cloned().collect();
            ApiResponse::ok(json!({ "domains": domains, "count": domains.len() })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_traffic(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            ApiResponse::ok(json!({ "traffic": c.traffic })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_attacks(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    let db_attacks = state.db.get_recent_attack_events(100);
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let attacks: Vec<_> = if c.attacks.is_empty() {
                db_attacks
            } else {
                c.attacks.iter().cloned().collect()
            };
            ApiResponse::ok(json!({ "attacks": attacks, "count": attacks.len() })).into_response()
        }
        None => ApiResponse::ok(json!({ "attacks": db_attacks, "count": db_attacks.len() }))
            .into_response(),
    }
}

pub async fn admin_monitoring_services(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let services: Vec<_> = c.services.values().cloned().collect();
            ApiResponse::ok(json!({ "services": services, "count": services.len() }))
                .into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_seo(
    operator: OperatorUser,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:read") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            let c = cache.read().await;
            let seo: Vec<_> = c.seo.values().cloned().collect();
            ApiResponse::ok(json!({ "seo": seo, "count": seo.len() })).into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}

pub async fn admin_monitoring_refresh(
    operator: OperatorUser,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Response {
    if let Err(resp) = require_perm(&operator, "settings:write") {
        return resp;
    }
    match &state.monitoring_cache {
        Some(cache) => {
            monitoring_service::force_refresh(
                state.config.port,
                &state.config.monitoring_node_ports,
                &state.config.monitoring_fail2ban_log_path,
                &state.config.monitoring_vhosts_path,
                &state.config.monitoring_nginx_log_path,
                &state.config.monitoring_cert_path,
                state.config.monitoring_retention_days,
                &state.db,
                cache,
            )
            .await;
            let client_ip = extract_client_ip(&headers);
            let _ = state.db.insert_audit_log(
                &operator.email,
                "monitoring_refresh",
                "monitoring",
                None,
                "{}",
                &client_ip,
            );
            ApiResponse::ok(json!({
                "refreshed": true,
                "message": "Sunucu izleme verileri güncellendi."
            }))
            .into_response()
        }
        None => json_error(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            msg_keys::SERVER_ERROR_GENERIC,
        ),
    }
}
