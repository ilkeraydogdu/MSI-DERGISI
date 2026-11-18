use chrono::Utc;
use super::types::SeoCheckResult;

/// Perform SEO checks for a domain.
pub async fn check_seo(domain: &str, client: &reqwest::Client) -> SeoCheckResult {
    let now = Utc::now().to_rfc3339();
    let mut issues = Vec::new();

    let robots_ok = match client
        .get(format!("https://{}/robots.txt", domain))
        .send()
        .await
    {
        Ok(r) => r.status().is_success(),
        Err(_) => {
            issues.push("robots.txt erişilemiyor".to_string());
            false
        }
    };

    let sitemap_ok = match client
        .get(format!("https://{}/sitemap.xml", domain))
        .send()
        .await
    {
        Ok(r) => r.status().is_success(),
        Err(_) => {
            issues.push("sitemap.xml erişilemiyor".to_string());
            false
        }
    };

    let (meta_title, meta_desc, canonical) =
        match client.get(format!("https://{}", domain)).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.text().await {
                    let lower = body.to_ascii_lowercase();
                    let has_title =
                        lower.contains("<title>") && !lower.contains("<title></title>");
                    let has_desc = lower.contains("name=\"description\"")
                        || lower.contains("name='description'");
                    let has_canonical =
                        lower.contains("rel=\"canonical\"") || lower.contains("rel='canonical'");
                    if !has_title {
                        issues.push("Eksik title etiketi".to_string());
                    }
                    if !has_desc {
                        issues.push("Eksik meta description".to_string());
                    }
                    if !has_canonical {
                        issues.push("Eksik canonical URL".to_string());
                    }
                    (has_title, has_desc, has_canonical)
                } else {
                    (false, false, false)
                }
            }
            Err(_) => {
                issues.push("Ana sayfa erişilemiyor".to_string());
                (false, false, false)
            }
        };

    SeoCheckResult {
        domain: domain.to_string(),
        robots_txt: robots_ok,
        sitemap_xml: sitemap_ok,
        meta_description: meta_desc,
        meta_title,
        canonical,
        issues,
        checked_at: now,
    }
}

